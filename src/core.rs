use tokio::time::Instant;

use crate::config::Config;
use std::{collections::{HashMap, HashSet, VecDeque}, sync::{LazyLock, Mutex}};

pub static CONFIG: LazyLock<Mutex<Config>> = LazyLock::new(|| {
    let config = Config::load_config();
    Mutex::new(config)
});

pub enum CacheValue {
    STR(String),
    List(VecDeque<T>),
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
