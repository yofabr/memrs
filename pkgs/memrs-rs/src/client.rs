use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use crate::error::{Error, Result};

const CRLF: &[u8] = b"\r\n";

/// An async TCP client for memrs.
///
/// Every method sends a single command and awaits the response before returning.
/// For connection reuse across tasks, wrap `Client` in an `Arc<Mutex<Client>>`
/// or use [`Pool`](crate::Pool).
///
/// # Example
///
/// ```no_run
/// use memrs_rs::Client;
///
/// # async fn run() -> memrs_rs::Result<()> {
/// let mut client = Client::connect("127.0.0.1:7898").await?;
/// client.auth("secret").await?;
/// client.set("key", "value").await?;
/// let val: String = client.get("key").await?;
/// assert_eq!(val, "value");
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct Client {
    writer: tokio::io::WriteHalf<TcpStream>,
    reader: BufReader<tokio::io::ReadHalf<TcpStream>>,
    addr: String,
    authenticated: bool,
}

impl Client {
    /// Connect to a memrs server at the given address.
    ///
    /// `addr` must be a valid socket address (e.g. `"127.0.0.1:7898"`).
    pub async fn connect(addr: &str) -> Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        let (reader, writer) = tokio::io::split(stream);
        Ok(Self {
            writer,
            reader: BufReader::new(reader),
            addr: addr.to_owned(),
            authenticated: false,
        })
    }

    /// Connect **and** authenticate in a single call.
    pub async fn connect_with_auth(addr: &str, password: &str) -> Result<Self> {
        let mut client = Self::connect(addr).await?;
        client.auth(password).await?;
        Ok(client)
    }

    /// The server address this client is connected to.
    pub fn addr(&self) -> &str {
        &self.addr
    }

    /// Whether the client has successfully authenticated.
    pub fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    /// Build a `Client` from already-separated IO halves.
    ///
    /// Useful when integrating with custom transport layers or for
    /// testing with mocked streams.
    pub fn from_raw_parts(
        writer: tokio::io::WriteHalf<TcpStream>,
        reader: BufReader<tokio::io::ReadHalf<TcpStream>>,
        addr: String,
    ) -> Self {
        Self {
            writer,
            reader,
            addr,
            authenticated: false,
        }
    }

    // ------------------------------------------------------------------
    // Low-level helpers
    // ------------------------------------------------------------------

    /// Send a raw command line and return the raw response line (trimmed).
    ///
    /// The trailing `\r\n` is appended automatically.
    /// Use this for commands the typed API does not yet cover.
    pub async fn run_command(&mut self, cmd: &str) -> Result<String> {
        self.writer.write_all(cmd.as_bytes()).await?;
        self.writer.write_all(CRLF).await?;

        let mut line = String::new();
        let n = self.reader.read_line(&mut line).await?;
        if n == 0 {
            return Err(Error::ConnectionClosed);
        }

        Ok(line.trim().to_owned())
    }

    fn check_error(raw: &str) -> Result<()> {
        if let Some(msg) = raw.strip_prefix("-ERR ") {
            return Err(Error::Protocol(msg.to_owned()));
        }
        if raw.strip_prefix("-NOAUTH ").is_some() {
            return Err(Error::AuthRequired);
        }
        Ok(())
    }

    fn expect_ok(raw: &str) -> Result<()> {
        Self::check_error(raw)?;
        if raw.starts_with('+') {
            return Ok(());
        }
        Err(Error::Protocol(format!("expected OK, got: {}", raw)))
    }

    fn value_response(raw: &str) -> Result<String> {
        Self::check_error(raw)?;
        Ok(raw.to_owned())
    }

    fn exists_response(raw: &str) -> Result<bool> {
        if raw == "+OK" {
            return Ok(true);
        }
        if let Some(msg) = raw.strip_prefix("-ERR ") {
            if msg == "No key found!" {
                return Ok(false);
            }
            return Err(Error::Protocol(msg.to_owned()));
        }
        Err(Error::Protocol(format!("unexpected EXISTS response: {}", raw)))
    }

    // ------------------------------------------------------------------
    // Connection & auth
    // ------------------------------------------------------------------

    /// Authenticate with the server.
    ///
    /// Returns an error if the password is rejected.
    pub async fn auth(&mut self, password: &str) -> Result<()> {
        let resp = self.run_command(&format!("AUTH {}", password)).await?;
        if resp == "+OK" {
            self.authenticated = true;
            Ok(())
        } else if let Some(msg) = resp.strip_prefix("-ERR ") {
            self.authenticated = false;
            Err(Error::AuthFailed(msg.to_owned()))
        } else {
            Err(Error::Protocol(format!("unexpected auth response: {}", resp)))
        }
    }

    /// Ping the server.
    ///
    /// Returns the server response (normally `"+PONG"`).
    pub async fn ping(&mut self) -> Result<String> {
        self.run_command("PING").await
    }

    // ------------------------------------------------------------------
    // String commands
    // ------------------------------------------------------------------

    /// Get the value of a key.
    ///
    /// Returns `Err(Error::NotFound)` when the key does not exist.
    pub async fn get(&mut self, key: &str) -> Result<String> {
        let resp = self.run_command(&format!("GET {}", key)).await?;
        if resp.starts_with("-ERR No records have been found") {
            return Err(Error::NotFound(key.to_owned()));
        }
        Self::check_error(&resp)?;
        Ok(resp)
    }

    /// Set a key to a string value.
    pub async fn set(&mut self, key: &str, value: &str) -> Result<()> {
        let resp = self.run_command(&format!("SET {} {}", key, value)).await?;
        Self::expect_ok(&resp)
    }

    /// Set a key with an optional TTL (in seconds).
    pub async fn set_ttl(&mut self, key: &str, value: &str, ttl: u64) -> Result<()> {
        let resp = self
            .run_command(&format!("SET {} {} {}", key, value, ttl))
            .await?;
        Self::expect_ok(&resp)
    }

    // ------------------------------------------------------------------
    // Key commands
    // ------------------------------------------------------------------

    /// Check if a key exists.
    pub async fn exists(&mut self, key: &str) -> Result<bool> {
        let resp = self.run_command(&format!("EXISTS {}", key)).await?;
        Self::exists_response(&resp)
    }

    /// Delete a key.
    pub async fn del(&mut self, key: &str) -> Result<()> {
        let resp = self.run_command(&format!("DEL {}", key)).await?;
        Self::expect_ok(&resp)
    }

    /// Delete all keys.
    pub async fn flushall(&mut self) -> Result<()> {
        let resp = self.run_command("FLUSHALL").await?;
        Self::expect_ok(&resp)
    }

    /// Set a timeout (in seconds) on a key.
    ///
    /// Returns `Err(Error::NotFound)` when the key does not exist.
    pub async fn expire(&mut self, key: &str, seconds: u64) -> Result<()> {
        let resp = self
            .run_command(&format!("EXPIRE {} {}", key, seconds))
            .await?;
        if resp.starts_with("-ERR No records have been found") {
            return Err(Error::NotFound(key.to_owned()));
        }
        Self::expect_ok(&resp)
    }

    /// List all keys with optional pagination (10 per page).
    pub async fn listall(&mut self, page: Option<usize>) -> Result<String> {
        let cmd = match page {
            Some(p) => format!("LISTALL {}", p),
            None => "LISTALL".to_string(),
        };
        let resp = self.run_command(&cmd).await?;
        Self::value_response(&resp)
    }

    // ------------------------------------------------------------------
    // Hash commands
    // ------------------------------------------------------------------

    /// Set the value of a field in a hash.
    pub async fn hset(&mut self, key: &str, field: &str, value: &str) -> Result<()> {
        let resp = self
            .run_command(&format!("HSET {} {} {}", key, field, value))
            .await?;
        Self::expect_ok(&resp)
    }

    /// Get the value of a field in a hash.
    pub async fn hget(&mut self, key: &str, field: &str) -> Result<String> {
        let resp = self
            .run_command(&format!("HGET {} {}", key, field))
            .await?;
        Self::check_error(&resp)?;
        if let Some(msg) = resp.strip_prefix("-ERR ") {
            return Err(Error::Protocol(msg.to_owned()));
        }
        Ok(resp)
    }

    // ------------------------------------------------------------------
    // List commands
    // ------------------------------------------------------------------

    /// Prepend a value to a list (left side).
    pub async fn lpush(&mut self, key: &str, value: &str) -> Result<()> {
        let resp = self
            .run_command(&format!("LPUSH {} {}", key, value))
            .await?;
        Self::expect_ok(&resp)
    }

    /// Append a value to a list (right side).
    pub async fn rpush(&mut self, key: &str, value: &str) -> Result<()> {
        let resp = self
            .run_command(&format!("RPUSH {} {}", key, value))
            .await?;
        Self::expect_ok(&resp)
    }

    /// Remove and return the first element of a list (left side).
    pub async fn lpop(&mut self, key: &str) -> Result<String> {
        let resp = self.run_command(&format!("LPOP {}", key)).await?;
        Self::check_error(&resp)?;
        if let Some(msg) = resp.strip_prefix("-ERR ") {
            return Err(Error::Protocol(msg.to_owned()));
        }
        Ok(resp)
    }

    /// Remove and return the last element of a list (right side).
    pub async fn rpop(&mut self, key: &str) -> Result<String> {
        let resp = self.run_command(&format!("RPOP {}", key)).await?;
        Self::check_error(&resp)?;
        if let Some(msg) = resp.strip_prefix("-ERR ") {
            return Err(Error::Protocol(msg.to_owned()));
        }
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;
    use tokio::net::TcpListener;

    /// Create a real TCP pair in the same process so we can mock the server.
    async fn fake_server() -> (Client, tokio::net::TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            stream
        });

        let client = Client::connect(&addr.to_string()).await.unwrap();
        let server = server.await.unwrap();
        (client, server)
    }

    /// Read one command line from `server` and write `response` back.
    /// Reads byte-by-byte until `\n` to guarantee exactly one line.
    async fn expect_cmd(server: &mut tokio::net::TcpStream, expected: &str, response: &str) {
        let mut buf = Vec::new();
        loop {
            let mut byte = [0u8; 1];
            server.read(&mut byte).await.unwrap();
            buf.push(byte[0]);
            if byte[0] == b'\n' {
                break;
            }
        }
        let got = std::str::from_utf8(&buf).unwrap();
        assert!(
            got.starts_with(expected),
            "expected cmd '{}', got '{}'",
            expected,
            got.trim()
        );
        server.write_all(response.as_bytes()).await.unwrap();
    }

    #[tokio::test]
    async fn ping() {
        let (mut client, mut server) = fake_server().await;
        tokio::spawn(async move {
            expect_cmd(&mut server, "PING", "+PONG\r\n").await;
        });
        let resp = client.ping().await.unwrap();
        assert_eq!(resp, "+PONG");
    }

    #[tokio::test]
    async fn set_and_get() {
        let (mut client, mut server) = fake_server().await;
        let server_handle = tokio::spawn(async move {
            expect_cmd(&mut server, "SET key value", "+OK\r\n").await;
            expect_cmd(&mut server, "GET key", "value\r\n").await;
        });
        client.set("key", "value").await.unwrap();
        let val = client.get("key").await.unwrap();
        assert_eq!(val, "value");
        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn get_not_found() {
        let (mut client, mut server) = fake_server().await;
        tokio::spawn(async move {
            expect_cmd(
                &mut server,
                "GET missing",
                "-ERR No records have been found\r\n",
            )
            .await;
        });
        let err = client.get("missing").await.unwrap_err();
        assert!(matches!(err, Error::NotFound(_)));
    }

    #[tokio::test]
    async fn set_ttl() {
        let (mut client, mut server) = fake_server().await;
        tokio::spawn(async move {
            expect_cmd(&mut server, "SET k v 100", "+OK\r\n").await;
        });
        client.set_ttl("k", "v", 100).await.unwrap();
    }

    #[tokio::test]
    async fn exists_true() {
        let (mut client, mut server) = fake_server().await;
        tokio::spawn(async move {
            expect_cmd(&mut server, "EXISTS k", "+OK\r\n").await;
        });
        assert!(client.exists("k").await.unwrap());
    }

    #[tokio::test]
    async fn exists_false() {
        let (mut client, mut server) = fake_server().await;
        tokio::spawn(async move {
            expect_cmd(&mut server, "EXISTS missing", "-ERR No key found!\r\n").await;
        });
        assert!(!client.exists("missing").await.unwrap());
    }

    #[tokio::test]
    async fn del_ok() {
        let (mut client, mut server) = fake_server().await;
        tokio::spawn(async move {
            expect_cmd(&mut server, "DEL k", "+OK\r\n").await;
        });
        client.del("k").await.unwrap();
    }

    #[tokio::test]
    async fn flushall_ok() {
        let (mut client, mut server) = fake_server().await;
        tokio::spawn(async move {
            expect_cmd(&mut server, "FLUSHALL", "+OK\r\n").await;
        });
        client.flushall().await.unwrap();
    }

    #[tokio::test]
    async fn expire_ok() {
        let (mut client, mut server) = fake_server().await;
        tokio::spawn(async move {
            expect_cmd(&mut server, "EXPIRE k 100", "+OK\r\n").await;
        });
        client.expire("k", 100).await.unwrap();
    }

    #[tokio::test]
    async fn expire_not_found() {
        let (mut client, mut server) = fake_server().await;
        tokio::spawn(async move {
            expect_cmd(
                &mut server,
                "EXPIRE missing 10",
                "-ERR No records have been found\r\n",
            )
            .await;
        });
        let err = client.expire("missing", 10).await.unwrap_err();
        assert!(matches!(err, Error::NotFound(_)));
    }

    #[tokio::test]
    async fn listall_default() {
        let (mut client, mut server) = fake_server().await;
        tokio::spawn(async move {
            expect_cmd(&mut server, "LISTALL", "+0 keys (Page 1/1)\r\n").await;
        });
        let resp = client.listall(None).await.unwrap();
        assert!(resp.contains("0 keys"));
    }

    #[tokio::test]
    async fn listall_with_page() {
        let (mut client, mut server) = fake_server().await;
        tokio::spawn(async move {
            expect_cmd(&mut server, "LISTALL 2", "+10 keys (Page 2/2)\r\n").await;
        });
        let resp = client.listall(Some(2)).await.unwrap();
        assert!(resp.contains("10 keys"));
    }

    #[tokio::test]
    async fn hset_hget() {
        let (mut client, mut server) = fake_server().await;
        let server_handle = tokio::spawn(async move {
            expect_cmd(&mut server, "HSET h f v", "+OK\r\n").await;
            expect_cmd(&mut server, "HGET h f", "v\r\n").await;
        });
        client.hset("h", "f", "v").await.unwrap();
        let val = client.hget("h", "f").await.unwrap();
        assert_eq!(val, "v");
        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn hget_not_found() {
        let (mut client, mut server) = fake_server().await;
        tokio::spawn(async move {
            expect_cmd(&mut server, "HGET h missing", "-ERR Field not found\r\n").await;
        });
        let err = client.hget("h", "missing").await.unwrap_err();
        assert!(matches!(err, Error::Protocol(_)));
    }

    #[tokio::test]
    async fn lpush_lpop() {
        let (mut client, mut server) = fake_server().await;
        let server_handle = tokio::spawn(async move {
            expect_cmd(&mut server, "LPUSH l a", "+OK\r\n").await;
            expect_cmd(&mut server, "LPUSH l b", "+OK\r\n").await;
            expect_cmd(&mut server, "LPOP l", "b\r\n").await;
            expect_cmd(&mut server, "LPOP l", "a\r\n").await;
        });
        client.lpush("l", "a").await.unwrap();
        client.lpush("l", "b").await.unwrap();
        assert_eq!(client.lpop("l").await.unwrap(), "b");
        assert_eq!(client.lpop("l").await.unwrap(), "a");
        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn rpush_rpop() {
        let (mut client, mut server) = fake_server().await;
        let server_handle = tokio::spawn(async move {
            expect_cmd(&mut server, "RPUSH l a", "+OK\r\n").await;
            expect_cmd(&mut server, "RPUSH l b", "+OK\r\n").await;
            expect_cmd(&mut server, "RPOP l", "b\r\n").await;
            expect_cmd(&mut server, "RPOP l", "a\r\n").await;
        });
        client.rpush("l", "a").await.unwrap();
        client.rpush("l", "b").await.unwrap();
        assert_eq!(client.rpop("l").await.unwrap(), "b");
        assert_eq!(client.rpop("l").await.unwrap(), "a");
        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn auth_correct() {
        let (mut client, mut server) = fake_server().await;
        tokio::spawn(async move {
            expect_cmd(&mut server, "AUTH secret", "+OK\r\n").await;
        });
        client.auth("secret").await.unwrap();
        assert!(client.is_authenticated());
    }

    #[tokio::test]
    async fn auth_wrong() {
        let (mut client, mut server) = fake_server().await;
        tokio::spawn(async move {
            expect_cmd(&mut server, "AUTH wrong", "-ERR invalid password\r\n").await;
        });
        let err = client.auth("wrong").await.unwrap_err();
        assert!(matches!(err, Error::AuthFailed(_)));
        assert!(!client.is_authenticated());
    }

    #[tokio::test]
    async fn protocol_error() {
        let (mut client, mut server) = fake_server().await;
        tokio::spawn(async move {
            expect_cmd(&mut server, "GET k", "-ERR something went wrong\r\n").await;
        });
        let err = client.get("k").await.unwrap_err();
        assert!(matches!(err, Error::Protocol(_)));
    }

    #[tokio::test]
    async fn noauth_error() {
        let (mut client, mut server) = fake_server().await;
        tokio::spawn(async move {
            expect_cmd(
                &mut server,
                "GET k",
                "-NOAUTH Authentication required\r\n",
            )
            .await;
        });
        let err = client.get("k").await.unwrap_err();
        assert!(matches!(err, Error::AuthRequired));
    }

    #[tokio::test]
    async fn connection_closed() {
        let (mut client, _server) = fake_server().await;
        drop(_server);
        let err = client.ping().await.unwrap_err();
        assert!(matches!(err, Error::ConnectionClosed) || matches!(err, Error::Io(_)));
    }

    #[tokio::test]
    async fn run_command_raw() {
        let (mut client, mut server) = fake_server().await;
        tokio::spawn(async move {
            expect_cmd(&mut server, "PING", "+PONG\r\n").await;
        });
        let resp = client.run_command("PING").await.unwrap();
        assert_eq!(resp, "+PONG");
    }
}
