//! # memrs-rs
//!
//! A comprehensive Rust client for **memrs** — an in-memory data store
//! with a Redis-compatible command set.
//!
//! ## Quick start
//!
//! ### Async (tokio)
//!
//! ```no_run
//! use memrs_rs::Client;
//!
//! # async fn run() -> memrs_rs::Result<()> {
//! let mut client = Client::connect("127.0.0.1:7898").await?;
//! client.set("hello", "world").await?;
//! let val = client.get("hello").await?;
//! assert_eq!(val, "world");
//! # Ok(())
//! # }
//! ```
//!
//! ### Sync (blocking)
//!
//! ```no_run
//! use memrs_rs::BlockingClient;
//!
//! let mut client = BlockingClient::connect("127.0.0.1:7898").unwrap();
//! client.set("hello", "world").unwrap();
//! let val = client.get("hello").unwrap();
//! assert_eq!(val, "world");
//! ```
//!
//! ### Connection pool
//!
//! ```no_run
//! use std::sync::Arc;
//! use memrs_rs::Pool;
//!
//! # async fn run() -> memrs_rs::Result<()> {
//! let pool = Arc::new(Pool::new("127.0.0.1:7898", 10));
//!
//! // Spawn many workers that share the pool
//! let pool_clone = pool.clone();
//! tokio::spawn(async move {
//!     let mut conn = pool_clone.get().await.unwrap();
//!     conn.set("key", "value").await.unwrap();
//!     // conn is returned to the pool on drop
//! });
//! # Ok(())
//! # }
//! ```
//!
//! ## Supported commands
//!
//! | Category  | Commands                                                      |
//! |-----------|---------------------------------------------------------------|
//! | Strings   | `GET`, `SET`, `SET … TTL`                                    |
//! | Keys      | `EXISTS`, `DEL`, `FLUSHALL`, `EXPIRE`, `LISTALL`             |
//! | Hashes    | `HSET`, `HGET`                                                |
//! | Lists     | `LPUSH`, `RPUSH`, `LPOP`, `RPOP`                              |
//! | Meta      | `PING`, `AUTH`                                                |
//! | Raw       | `run_command`                                                 |

pub mod blocking;
pub mod client;
pub mod error;
pub mod pool;

pub use blocking::BlockingClient;
pub use client::Client;
pub use error::{Error, Result};
pub use pool::{Pool, PooledClient};
