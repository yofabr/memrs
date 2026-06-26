use color_eyre::Result;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

use crate::core::{CONFIG, STORE};
use crate::repl::ReplCommands;

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
    let port_number = CONFIG.lock().unwrap().port;
    let address = format!("0.0.0.0:{}", port_number);
    let listener = TcpListener::bind(&address).await?;

    println!("[INFO] Started TcpListener at: {}", &address);
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

async fn process_command(client: &mut Client, cmd: &str) -> String {
    let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
    let command = parts[0].to_uppercase();

    match command.as_str() {
        "PING" => "+PONG".to_string(),
        "AUTH" => {
            let config = CONFIG.lock().unwrap();
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
                Ok(parsed) => match STORE.lock().unwrap().execute(parsed) {
                    Ok(response) => response,
                    Err(e) => format!("-ERR {}", e),
                },
                Err(e) => format!("-ERR {}", e),
            }
        }
    }
}
