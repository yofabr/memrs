use crate::client::Client;
use crate::error::Result;

/// Synchronous (blocking) client for memrs.
///
/// Wraps the async [`Client`] in a single-threaded tokio runtime so
/// every method blocks the current thread until the server responds.
///
/// Useful where `async`/`.await` is not available (simple scripts,
/// sync-only libraries, etc.).
///
/// # Example
///
/// ```no_run
/// use memrs_rs::BlockingClient;
///
/// let mut client = BlockingClient::connect("127.0.0.1:7898").unwrap();
/// client.auth("secret").unwrap();
/// client.set("key", "value").unwrap();
/// let val = client.get("key").unwrap();
/// assert_eq!(val, "value");
/// ```
#[derive(Debug)]
pub struct BlockingClient {
    inner: Client,
    rt: tokio::runtime::Runtime,
}

impl BlockingClient {
    /// Connect to a memrs server, blocking the current thread.
    pub fn connect(addr: &str) -> Result<Self> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .map_err(crate::error::Error::Io)?;
        let inner = rt.block_on(Client::connect(addr))?;
        Ok(Self { inner, rt })
    }

    /// Connect and authenticate in one call.
    pub fn connect_with_auth(addr: &str, password: &str) -> Result<Self> {
        let mut this = Self::connect(addr)?;
        this.auth(password)?;
        Ok(this)
    }

    /// The server address this client is connected to.
    pub fn addr(&self) -> &str {
        self.inner.addr()
    }

    /// Whether the client has authenticated.
    pub fn is_authenticated(&self) -> bool {
        self.inner.is_authenticated()
    }

    // ------------------------------------------------------------------
    // Commands
    // ------------------------------------------------------------------

    /// Authenticate.
    pub fn auth(&mut self, password: &str) -> Result<()> {
        self.rt.block_on(self.inner.auth(password))
    }

    /// Ping the server.
    pub fn ping(&mut self) -> Result<String> {
        self.rt.block_on(self.inner.ping())
    }

    /// Get the value of a key.
    pub fn get(&mut self, key: &str) -> Result<String> {
        self.rt.block_on(self.inner.get(key))
    }

    /// Set a key to a string value.
    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        self.rt.block_on(self.inner.set(key, value))
    }

    /// Set a key with a TTL in seconds.
    pub fn set_ttl(&mut self, key: &str, value: &str, ttl: u64) -> Result<()> {
        self.rt.block_on(self.inner.set_ttl(key, value, ttl))
    }

    /// Check if a key exists.
    pub fn exists(&mut self, key: &str) -> Result<bool> {
        self.rt.block_on(self.inner.exists(key))
    }

    /// Delete a key.
    pub fn del(&mut self, key: &str) -> Result<()> {
        self.rt.block_on(self.inner.del(key))
    }

    /// Delete all keys.
    pub fn flushall(&mut self) -> Result<()> {
        self.rt.block_on(self.inner.flushall())
    }

    /// Set a timeout on a key.
    pub fn expire(&mut self, key: &str, seconds: u64) -> Result<()> {
        self.rt.block_on(self.inner.expire(key, seconds))
    }

    /// List all keys with optional pagination.
    pub fn listall(&mut self, page: Option<usize>) -> Result<String> {
        self.rt.block_on(self.inner.listall(page))
    }

    /// Set a field in a hash.
    pub fn hset(&mut self, key: &str, field: &str, value: &str) -> Result<()> {
        self.rt.block_on(self.inner.hset(key, field, value))
    }

    /// Get a field from a hash.
    pub fn hget(&mut self, key: &str, field: &str) -> Result<String> {
        self.rt.block_on(self.inner.hget(key, field))
    }

    /// Prepend to a list.
    pub fn lpush(&mut self, key: &str, value: &str) -> Result<()> {
        self.rt.block_on(self.inner.lpush(key, value))
    }

    /// Append to a list.
    pub fn rpush(&mut self, key: &str, value: &str) -> Result<()> {
        self.rt.block_on(self.inner.rpush(key, value))
    }

    /// Pop from the left of a list.
    pub fn lpop(&mut self, key: &str) -> Result<String> {
        self.rt.block_on(self.inner.lpop(key))
    }

    /// Pop from the right of a list.
    pub fn rpop(&mut self, key: &str) -> Result<String> {
        self.rt.block_on(self.inner.rpop(key))
    }

    /// Send a raw command.
    pub fn run_command(&mut self, cmd: &str) -> Result<String> {
        self.rt.block_on(self.inner.run_command(cmd))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_fails_on_bad_addr() {
        let result = BlockingClient::connect("127.0.0.1:1");
        assert!(result.is_err());
        assert!(matches!(result, Err(crate::error::Error::Io(_))));
    }

    #[test]
    fn client_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<BlockingClient>();
    }
}
