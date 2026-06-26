use crate::{config::Config, repl::ReplCommands};
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
    fn lpush(&mut self, key: String, value: String) -> Result<()>;

    // Inserts a new element at the tail of the list
    fn rpush(&mut self, key: String, value: String) -> Result<()>;

    // Removes and returns the first element from the left
    fn lpop(&mut self, key: String) -> Result<String>;

    // Removes and returns the last element from the right
    fn rpop(&mut self, key: String) -> Result<String>;

    // returns a range of items
    fn lrange(&self, key: String);
}

pub trait HashOps {
    // Sets the value of a specific field within a hash
    fn hset(&mut self, key: String, field: String, value: String) -> Result<()>;

    // Retrieves the value of a specific field
    fn hget(&self, key: String, field: String) -> Result<String>;
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
    pub created_at: Instant,
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

impl Store {
    pub fn execute(&mut self, command: ReplCommands) -> Result<String> {
        match command {
            ReplCommands::GET(key) => {
                let entry = self.get(key)?;
                match &entry.item {
                    CacheValue::STR(s) => Ok(s.clone()),
                    other => Ok(format!("{:?}", other)),
                }
            }
            ReplCommands::SET(key, value) => {
                self.set(
                    key,
                    CacheEntry {
                        item: CacheValue::STR(value),
                        ttl: None,
                        created_at: Instant::now(),
                    },
                )?;
                Ok("+OK".to_string())
            }
            ReplCommands::EXISTS(key) => {
                self.exists(key)?;
                Ok("+OK".to_string())
            }
            ReplCommands::DEL(key) => {
                self.del(key)?;
                Ok("+OK".to_string())
            }
            ReplCommands::HSET(key, field, value) => {
                self.hset(key, field, value)?;
                Ok("+OK".to_string())
            }
            ReplCommands::HGET(key, field) => {
                let val = self.hget(key, field)?;
                Ok(val)
            }
            ReplCommands::LPUSH(key, value) => {
                self.lpush(key, value)?;
                Ok("+OK".to_string())
            }
            ReplCommands::RPUSH(key, value) => {
                self.rpush(key, value)?;
                Ok("+OK".to_string())
            }
            ReplCommands::LPOP(key) => {
                let val = self.lpop(key)?;
                Ok(val)
            }
            ReplCommands::RPOP(key) => {
                let val = self.rpop(key)?;
                Ok(val)
            }
            ReplCommands::PING => Ok("+PONG".to_string()),
            ReplCommands::FLUSHALL => {
                self.flushall()?;
                Ok("+OK".to_string())
            }
            ReplCommands::LISTALL(page) => {
                let page = page.unwrap_or(1).max(1);
                let limit = 10;
                let mut entries: Vec<(&String, &CacheEntry)> = self.data.iter().collect();
                entries.sort_by(|a, b| b.1.created_at.cmp(&a.1.created_at));
                let total = entries.len();
                let total_pages = total.div_ceil(limit).max(1);
                let start = (page - 1) * limit;
                let batch: Vec<&(&String, &CacheEntry)> = entries.iter().skip(start).take(limit).collect();

                if batch.is_empty() {
                    Ok(format!("+0 keys (Page {}/{})", page, total_pages))
                } else {
                    let items: Vec<String> = batch
                        .iter()
                        .map(|(k, v)| {
                            let type_str = match &v.item {
                                CacheValue::STR(_) => "STRING".to_string(),
                                CacheValue::List(l) => format!("LIST[{}]", l.len()),
                                CacheValue::SET(s) => format!("SET[{}]", s.len()),
                                CacheValue::Map(m) => format!("HASH[{}]", m.len()),
                            };
                            format!("{} ({})", k, type_str)
                        })
                        .collect();
                    Ok(format!("+{} keys (Page {}/{}): {}", total, page, total_pages, items.join(", ")))
                }
            }
        }
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

impl ListOps for Store {
    fn lpush(&mut self, key: String, value: String) -> Result<()> {
        let entry = self.data.entry(key).or_insert_with(|| CacheEntry {
            item: CacheValue::List(VecDeque::new()),
            ttl: None,
            created_at: Instant::now(),
        });
        match &mut entry.item {
            CacheValue::List(list) => {
                list.push_front(value);
                Ok(())
            }
            _ => Err(eyre!("Key is not a list")),
        }
    }

    fn rpush(&mut self, key: String, value: String) -> Result<()> {
        let entry = self.data.entry(key).or_insert_with(|| CacheEntry {
            item: CacheValue::List(VecDeque::new()),
            ttl: None,
            created_at: Instant::now(),
        });
        match &mut entry.item {
            CacheValue::List(list) => {
                list.push_back(value);
                Ok(())
            }
            _ => Err(eyre!("Key is not a list")),
        }
    }

    fn lpop(&mut self, key: String) -> Result<String> {
        let entry = self.data.get_mut(&key);
        match entry {
            Some(entry) => match &mut entry.item {
                CacheValue::List(list) => list.pop_front().ok_or_else(|| eyre!("List is empty")),
                _ => Err(eyre!("Key is not a list")),
            },
            None => Err(eyre!("No records have been found")),
        }
    }

    fn rpop(&mut self, key: String) -> Result<String> {
        let entry = self.data.get_mut(&key);
        match entry {
            Some(entry) => match &mut entry.item {
                CacheValue::List(list) => list.pop_back().ok_or_else(|| eyre!("List is empty")),
                _ => Err(eyre!("Key is not a list")),
            },
            None => Err(eyre!("No records have been found")),
        }
    }

    fn lrange(&self, _key: String) {
        todo!()
    }
}

impl HashOps for Store {
    fn hset(&mut self, key: String, field: String, value: String) -> Result<()> {
        let entry = self.data.entry(key).or_insert_with(|| CacheEntry {
            item: CacheValue::Map(HashMap::new()),
            ttl: None,
            created_at: Instant::now(),
        });
        match &mut entry.item {
            CacheValue::Map(map) => {
                map.insert(field, value);
                Ok(())
            }
            _ => Err(eyre!("Key is not a hash")),
        }
    }

    fn hget(&self, key: String, field: String) -> Result<String> {
        let entry = self.data.get(&key);
        match entry {
            Some(entry) => match &entry.item {
                CacheValue::Map(map) => map
                    .get(&field)
                    .cloned()
                    .ok_or_else(|| eyre!("Field not found")),
                _ => Err(eyre!("Key is not a hash")),
            },
            None => Err(eyre!("No records have been found")),
        }
    }
}

pub static STORE: LazyLock<Mutex<Store>> = LazyLock::new(|| {
    let store: HashMap<String, CacheEntry> = HashMap::new();
    let store_constructor = Store::new(store);
    Mutex::new(store_constructor)
});
