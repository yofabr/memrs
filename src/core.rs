use tokio::time::Instant;

use crate::config::Config;
use std::{collections::{HashMap, HashSet, VecDeque}, sync::{LazyLock, Mutex}, time::Duration};

pub static CONFIG: LazyLock<Mutex<Config>> = LazyLock::new(|| {
    let config = Config::load_config();
    Mutex::new(config)
});

pub trait BasicOps {
    // Creates or updates a key in a database
    fn set(&self, key: String, value: CacheValue);

    // Returns the value of a key
    fn get(&self, key: String);
}

pub trait KeyOps {
    // Checks if a key exists in a db
    fn exists(&self, key: String);

    // Deletes a key from a db
    fn del(&self, key: String);
    
    // Frees up db (be careful when using this in a production code)
    fn flushall(&self);

    // List all the keys matching the passed pattern
    fn keys(&self, pattern: String);

    // Sets an expiry for a key
    fn expire(&self, key: String, time: Duration);
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


pub enum CacheValue {
    STR(String),
    List(VecDeque<String>),
    SET(HashSet<String>),
    Map(HashMap<String, String>)
}

pub struct CacheEntry {
    pub item: CacheValue,
    pub ttl: Option<Instant>
}

pub struct Store {
    data: HashMap<String, CacheEntry>
}

impl Store {
    fn new(store: HashMap<String, CacheEntry>) -> Self {
        Self {
            data: store
        }
    }
}

pub static STORE: LazyLock<Mutex<Store> = LazyLock::new(|| {
    let store: HashMap<String, CacheEntry> = HashMap::new();
    let store_constructor = Store::new(store);
    Mutex::new(store_constructor)
});
