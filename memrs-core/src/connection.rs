use color_eyre::Result;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

use crate::core::{start_expiry_worker, CONFIG, STORE};
use crate::repl::ReplCommands;
use crate::snapshot;

#[derive(Debug, Clone, PartialEq)]
pub enum ClientType {
    CLI,
    APPLICATION,
}

#[derive(Debug, Clone)]
pub struct Client {
    pub is_authenticated: bool,
    pub client_type: ClientType,
}

impl Client {
    pub fn new() -> Self {
        Self {
            is_authenticated: false,
            client_type: ClientType::CLI,
        }
    }
}

pub async fn init_listener() -> Result<()> {
    let port_number = CONFIG.read().port;
    let address = format!("0.0.0.0:{}", port_number);
    let listener = TcpListener::bind(&address).await?;

    snapshot::try_load_at_startup();
    println!("[INFO] Started TcpListener at: {}", &address);
    tokio::spawn(async { start_expiry_worker().await });
    tokio::spawn(async { snapshot::start_snapshot_worker().await });
    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream).await {
                eprintln!("[ERROR] Connection error: {}", e);
            }
        });
    }
}

pub async fn handle_connection(mut stream: TcpStream) -> Result<()> {
    let mut client = Client::new();
    let (reader, mut writer) = stream.split();
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = buf_reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            break;
        }

        let cmd = line.trim();
        if cmd.is_empty() {
            continue;
        }

        let response = process_command(&mut client, cmd).await;
        writer.write_all(response.as_bytes()).await?;
        writer.write_all(b"\r\n").await?;
    }

    Ok(())
}

pub(crate) async fn process_command(client: &mut Client, cmd: &str) -> String {
    let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
    let command = parts[0].to_uppercase();

    match command.as_str() {
        "PING" => "+PONG".to_string(),
        "AUTH" => {
            let config = CONFIG.read();
            if config.password.is_empty() {
                client.is_authenticated = true;
                "+OK".to_string()
            } else if let Some(pass) = parts.get(1) {
                if config.verify_password(pass.trim()) {
                    client.is_authenticated = true;
                    "+OK".to_string()
                } else {
                    client.is_authenticated = false;
                    "-ERR invalid password".to_string()
                }
            } else {
                "-ERR wrong number of arguments for 'AUTH' command".to_string()
            }
        }
        _ => {
            if !client.is_authenticated {
                return "-NOAUTH Authentication required".to_string();
            }
            match ReplCommands::parse_command(cmd.to_string()) {
                Ok(parsed) => match STORE.write().execute(parsed) {
                    Ok(response) => response,
                    Err(e) => format!("-ERR {}", e),
                },
                Err(e) => format!("-ERR {}", e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::core::KeyOps;
    use std::sync::Mutex;

    /// Serialises connection tests that touch the global CONFIG / STORE statics.
    static SERIAL: Mutex<()> = Mutex::new(());

    fn setup_client() -> Client {
        Client::new()
    }

    fn hash_pass(pass: &str) -> String {
        Config::hash_password(pass).unwrap()
    }

    fn set_config_password(password: String) -> String {
        let mut config = CONFIG.write();
        let old = config.password.clone();
        config.password = password;
        old
    }

    fn reset_config_password(old: String) {
        CONFIG.write().password = old;
    }

    fn reset_store() {
        STORE.write().flushall().unwrap();
    }

    #[tokio::test]
    async fn process_ping() {
        let _guard = SERIAL.lock().unwrap();
        let mut client = setup_client();
        let resp = process_command(&mut client, "PING").await;
        assert_eq!(resp, "+PONG");
    }

    #[tokio::test]
    async fn process_auth_no_password() {
        let _guard = SERIAL.lock().unwrap();
        let mut client = setup_client();
        let old = set_config_password(String::new());
        let resp = process_command(&mut client, "AUTH").await;
        assert_eq!(resp, "+OK");
        assert!(client.is_authenticated);
        reset_config_password(old);
    }

    #[tokio::test]
    async fn process_auth_correct_password() {
        let _guard = SERIAL.lock().unwrap();
        let mut client = setup_client();
        let old = set_config_password(hash_pass("secret"));
        let resp = process_command(&mut client, "AUTH secret").await;
        assert_eq!(resp, "+OK");
        assert!(client.is_authenticated);
        reset_config_password(old);
    }

    #[tokio::test]
    async fn process_auth_wrong_password() {
        let _guard = SERIAL.lock().unwrap();
        let mut client = setup_client();
        let old = set_config_password(hash_pass("secret"));
        let resp = process_command(&mut client, "AUTH wrongpass").await;
        assert_eq!(resp, "-ERR invalid password");
        assert!(!client.is_authenticated);
        reset_config_password(old);
    }

    #[tokio::test]
    async fn process_auth_missing_password_arg() {
        let _guard = SERIAL.lock().unwrap();
        let mut client = setup_client();
        let old = set_config_password(hash_pass("secret"));
        let resp = process_command(&mut client, "AUTH").await;
        assert_eq!(resp, "-ERR wrong number of arguments for 'AUTH' command");
        assert!(!client.is_authenticated);
        reset_config_password(old);
    }

    #[tokio::test]
    async fn process_noauth_blocks_command() {
        let _guard = SERIAL.lock().unwrap();
        let mut client = setup_client();
        let old = set_config_password(hash_pass("secret"));
        let resp = process_command(&mut client, "SET k v").await;
        assert_eq!(resp, "-NOAUTH Authentication required");
        assert!(!client.is_authenticated);
        reset_config_password(old);
    }

    #[tokio::test]
    async fn process_auth_then_command() {
        let _guard = SERIAL.lock().unwrap();
        let mut client = setup_client();
        let old = set_config_password(hash_pass("secret"));
        reset_store();

        let auth_resp = process_command(&mut client, "AUTH secret").await;
        assert_eq!(auth_resp, "+OK");
        assert!(client.is_authenticated);

        let set_resp = process_command(&mut client, "SET mykey myvalue").await;
        assert_eq!(set_resp, "+OK");

        let get_resp = process_command(&mut client, "GET mykey").await;
        assert_eq!(get_resp, "myvalue");

        reset_config_password(old);
        reset_store();
    }
}
