use crate::config::Config;
use color_eyre::{eyre::eyre, Result};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{LazyLock, Mutex},
    // time::Duration,
};
use tokio::time::Instant;

pub static CONFIG: LazyLock<Mutex<Config>> = LazyLock::new(|| {
    let config = Config::load_config();
    Mutex::new(config)
});

pub trait BasicOps {
    // Creates or updates a key in a database
    fn set(&mut self, key: String, value: CacheEntry) -> Result<()>;

    // Returns the value of a key
    fn get(&self, key: String) -> Result<CacheEntry>;
}

pub trait KeyOps {
    // Checks if a key exists in a db
    fn exists(&self, key: String) -> Result<()>;

    // Deletes a key from a db
    fn del(&mut self, key: String) -> Result<()>;

    // Frees up db (be careful when using this in a production code)
    fn flushall(&mut self) -> Result<()>;

    // List all the keys matching the passed pattern
    // fn keys(&self, pattern: String);

    // Sets an expiry for a key
    // fn expire(&self, key: String, time: Duration);
}

pub trait ListOps {
    // Inserts a new element at the head of the list
    fn lpush(&self, key: String, value: String);

    // Inserts a new element at the tail of the list
    fn rpush(&self, key: String, value: String);

    // returns a range of items
    fn lrange(&self, key: String);
}

pub trait SetOps {
    // Adds an element to a set
    fn sadd(&self, key: String, member: String);

    // Lists every item contained in a list
    fn smembers(&self, key: String);
}

#[derive(Debug, Clone)]
pub enum CacheValue {
    STR(String),
    List(VecDeque<String>),
    SET(HashSet<String>),
    Map(HashMap<String, String>),
}

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub item: CacheValue,
    pub ttl: Option<Instant>,
}

#[derive(Debug, Clone)]
pub struct Store {
    data: HashMap<String, CacheEntry>,
}

impl Store {
    fn new(store: HashMap<String, CacheEntry>) -> Self {
        Self { data: store }
    }
}

impl BasicOps for Store {
    fn set(&mut self, key: String, value: CacheEntry) -> Result<()> {
        self.data.insert(key, value);
        Ok(())
    }

    fn get(&self, key: String) -> Result<CacheEntry> {
        // This will check for expiry dates on each get in the future.
        let entry = self.data.get(&key);
        if let Some(entry) = entry {
            Ok(entry.clone())
        } else {
            Err(eyre!("No records have been found"))
        }
    }
}

impl KeyOps for Store {
    fn exists(&self, key: String) -> Result<()> {
        if self.data.contains_key(&key) {
            Ok(())
        } else {
            Err(eyre!("No key found!"))
        }
    }

    fn del(&mut self, key: String) -> Result<()> {
        // More logics will happen on this one
        let _res = self.data.remove(&key);
        Ok(())
    }

    fn flushall(&mut self) -> Result<()> {
        self.data.clear();
        Ok(())
    }
}

pub static STORE: LazyLock<Mutex<Store>> = LazyLock::new(|| {
    let store: HashMap<String, CacheEntry> = HashMap::new();
    let store_constructor = Store::new(store);
    Mutex::new(store_constructor)
});
