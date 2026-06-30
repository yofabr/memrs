use std::sync::Arc;

use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};

use crate::client::Client;
use crate::error::Result;

/// A connection pool for memrs.
///
/// Manages a set of reusable [`Client`] connections.  Clients are
/// checked out via [`Pool::get`] and returned automatically when the
/// [`PooledClient`] is dropped.
///
/// The pool also supports **polling**: call [`Pool::health`] to check
/// whether any idle connection can reach the server.
///
/// `Pool` is cheaply cloneable (internally reference-counted).
///
/// # Example
///
/// ```no_run
/// use std::sync::Arc;
/// use memrs_rs::Pool;
///
/// # async fn run() -> memrs_rs::Result<()> {
/// let pool = Arc::new(Pool::new("127.0.0.1:7898", 5));
/// let mut client = pool.get().await?;
/// client.set("key", "value").await?;
/// // client is returned to the pool when it goes out of scope
/// # Ok(())
/// # }
/// ```
pub struct Pool {
    inner: Arc<PoolInner>,
}

struct PoolInner {
    addr: String,
    semaphore: Arc<Semaphore>,
    clients: Mutex<Vec<Client>>,
}

impl Clone for Pool {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl Pool {
    /// Create a new pool that manages up to `max_size` connections to `addr`.
    pub fn new(addr: &str, max_size: usize) -> Self {
        Self {
            inner: Arc::new(PoolInner {
                addr: addr.to_owned(),
                semaphore: Arc::new(Semaphore::new(max_size)),
                clients: Mutex::new(Vec::with_capacity(max_size)),
            }),
        }
    }

    /// The server address the pool connects to.
    pub fn addr(&self) -> &str {
        &self.inner.addr
    }

    /// Check out a connection from the pool.
    ///
    /// If an idle connection is available it is returned immediately;
    /// otherwise a new connection is established.
    pub async fn get(&self) -> Result<PooledClient> {
        let permit = self.inner.semaphore.clone().acquire_owned().await?;
        let mut guard = self.inner.clients.lock().await;
        let client = match guard.pop() {
            Some(c) => c,
            None => Client::connect(&self.inner.addr).await?,
        };
        drop(guard);

        Ok(PooledClient {
            client: Some(client),
            pool: self.clone(),
            _permit: permit,
        })
    }

    /// Return a connection to the pool (called by `PooledClient::drop`).
    async fn put_back(&self, client: Client) {
        self.inner.clients.lock().await.push(client);
    }

    /// Poll / health-check: borrow an idle connection (if one exists)
    /// and send a PING.
    ///
    /// Returns `true` if at least one idle connection responded
    /// successfully, `false` otherwise.
    pub async fn health(&self) -> bool {
        let mut client = match self.inner.clients.lock().await.pop() {
            Some(c) => c,
            None => return false,
        };
        let ok = client.ping().await;
        self.put_back(client).await;
        ok.is_ok()
    }

    /// Number of connections currently idle in the pool.
    pub async fn idle_count(&self) -> usize {
        self.inner.clients.lock().await.len()
    }

    /// Maximum number of connections this pool was configured with.
    pub fn max_size(&self) -> usize {
        self.inner.semaphore.available_permits()
            + self
                .inner
                .clients
                .try_lock()
                .map(|g| g.len())
                .unwrap_or(0)
    }
}

/// A checked-out connection from a [`Pool`].
///
/// Provides the same command API as [`Client`].  When dropped the
/// underlying connection is returned to the pool for reuse.
pub struct PooledClient {
    client: Option<Client>,
    pool: Pool,
    _permit: OwnedSemaphorePermit,
}

impl PooledClient {
    fn client(&mut self) -> &mut Client {
        self.client.as_mut().expect("PooledClient used after drop")
    }

    /// Ping the server through this connection.
    pub async fn ping(&mut self) -> Result<String> {
        self.client().ping().await
    }

    /// Authenticate on this connection.
    pub async fn auth(&mut self, password: &str) -> Result<()> {
        self.client().auth(password).await
    }

    /// Whether the underlying client has authenticated.
    pub fn is_authenticated(&self) -> bool {
        self.client
            .as_ref()
            .map(|c| c.is_authenticated())
            .unwrap_or(false)
    }

    /// Get the value of a key.
    pub async fn get(&mut self, key: &str) -> Result<String> {
        self.client().get(key).await
    }

    /// Set a key to a string value.
    pub async fn set(&mut self, key: &str, value: &str) -> Result<()> {
        self.client().set(key, value).await
    }

    /// Set a key with a TTL in seconds.
    pub async fn set_ttl(&mut self, key: &str, value: &str, ttl: u64) -> Result<()> {
        self.client().set_ttl(key, value, ttl).await
    }

    /// Check if a key exists.
    pub async fn exists(&mut self, key: &str) -> Result<bool> {
        self.client().exists(key).await
    }

    /// Delete a key.
    pub async fn del(&mut self, key: &str) -> Result<()> {
        self.client().del(key).await
    }

    /// Delete all keys.
    pub async fn flushall(&mut self) -> Result<()> {
        self.client().flushall().await
    }

    /// Set a timeout on a key.
    pub async fn expire(&mut self, key: &str, seconds: u64) -> Result<()> {
        self.client().expire(key, seconds).await
    }

    /// List all keys with optional pagination.
    pub async fn listall(&mut self, page: Option<usize>) -> Result<String> {
        self.client().listall(page).await
    }

    /// Set a field in a hash.
    pub async fn hset(&mut self, key: &str, field: &str, value: &str) -> Result<()> {
        self.client().hset(key, field, value).await
    }

    /// Get a field from a hash.
    pub async fn hget(&mut self, key: &str, field: &str) -> Result<String> {
        self.client().hget(key, field).await
    }

    /// Prepend to a list.
    pub async fn lpush(&mut self, key: &str, value: &str) -> Result<()> {
        self.client().lpush(key, value).await
    }

    /// Append to a list.
    pub async fn rpush(&mut self, key: &str, value: &str) -> Result<()> {
        self.client().rpush(key, value).await
    }

    /// Pop from the left of a list.
    pub async fn lpop(&mut self, key: &str) -> Result<String> {
        self.client().lpop(key).await
    }

    /// Pop from the right of a list.
    pub async fn rpop(&mut self, key: &str) -> Result<String> {
        self.client().rpop(key).await
    }
}

impl Drop for PooledClient {
    fn drop(&mut self) {
        if let Some(client) = self.client.take() {
            let pool = self.pool.clone();
            tokio::spawn(async move {
                pool.put_back(client).await;
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pool_create_and_get() {
        let pool = Pool::new("127.0.0.1:9999", 2);
        assert_eq!(pool.max_size(), 2);
        assert_eq!(pool.idle_count().await, 0);
    }

    #[tokio::test]
    async fn pool_health_false_when_empty() {
        let pool = Pool::new("127.0.0.1:9999", 2);
        assert!(!pool.health().await);
    }

    #[tokio::test]
    async fn pool_clone_is_cheap() {
        let pool = Pool::new("127.0.0.1:7898", 5);
        let cloned = pool.clone();
        assert_eq!(pool.max_size(), cloned.max_size());
    }

    #[test]
    fn pool_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<Pool>();
        assert_sync::<Pool>();
    }
}
