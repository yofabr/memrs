use crate::{config::Config, repl::ReplCommands};
use color_eyre::{eyre::eyre, Result};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::LazyLock,
    time::Duration,
};
use tokio::time::Instant;
use parking_lot::RwLock;

pub static CONFIG: LazyLock<RwLock<Config>> = LazyLock::new(|| {
    let config = Config::load_config();
    RwLock::new(config)
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
    entry_expiries: HashMap<String, Instant>,
}

impl Store {
    fn new(store: HashMap<String, CacheEntry>) -> Self {
        Self {
            data: store,
            entry_expiries: HashMap::new(),
        }
    }

    fn check_expiry(&mut self, key: &str) {
        if let Some(entry) = self.data.get(key) {
            if let Some(deadline) = entry.ttl {
                if Instant::now() >= deadline {
                    self.data.remove(key);
                    self.entry_expiries.remove(key);
                }
            }
        }
    }
}

impl Store {
    pub fn execute(&mut self, command: ReplCommands) -> Result<String> {
        match command {
            ReplCommands::GET(key) => {
                self.check_expiry(&key);
                let entry = self.get(key)?;
                match &entry.item {
                    CacheValue::STR(s) => Ok(s.clone()),
                    other => Ok(format!("{:?}", other)),
                }
            }
            ReplCommands::SET(key, value, ttl) => {
                let deadline = ttl.map(|secs| Instant::now() + Duration::from_secs(secs));
                if let Some(d) = deadline {
                    self.entry_expiries.insert(key.clone(), d);
                }
                self.set(
                    key,
                    CacheEntry {
                        item: CacheValue::STR(value),
                        ttl: deadline,
                        created_at: Instant::now(),
                    },
                )?;
                Ok("+OK".to_string())
            }
            ReplCommands::EXISTS(key) => {
                self.check_expiry(&key);
                self.exists(key)?;
                Ok("+OK".to_string())
            }
            ReplCommands::DEL(key) => {
                self.del(key)?;
                Ok("+OK".to_string())
            }
            ReplCommands::HSET(key, field, value) => {
                self.check_expiry(&key);
                self.hset(key, field, value)?;
                Ok("+OK".to_string())
            }
            ReplCommands::HGET(key, field) => {
                self.check_expiry(&key);
                let val = self.hget(key, field)?;
                Ok(val)
            }
            ReplCommands::LPUSH(key, value) => {
                self.check_expiry(&key);
                self.lpush(key, value)?;
                Ok("+OK".to_string())
            }
            ReplCommands::RPUSH(key, value) => {
                self.check_expiry(&key);
                self.rpush(key, value)?;
                Ok("+OK".to_string())
            }
            ReplCommands::LPOP(key) => {
                self.check_expiry(&key);
                let val = self.lpop(key)?;
                Ok(val)
            }
            ReplCommands::RPOP(key) => {
                self.check_expiry(&key);
                let val = self.rpop(key)?;
                Ok(val)
            }
            ReplCommands::PING => Ok("+PONG".to_string()),
            ReplCommands::EXPIRE(key, seconds) => {
                let deadline = Instant::now() + Duration::from_secs(seconds);
                if let Some(entry) = self.data.get_mut(&key) {
                    entry.ttl = Some(deadline);
                    self.entry_expiries.insert(key, deadline);
                    Ok("+OK".to_string())
                } else {
                    Err(eyre!("No records have been found"))
                }
            }
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
                let batch: Vec<&(&String, &CacheEntry)> =
                    entries.iter().skip(start).take(limit).collect();

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
                    Ok(format!(
                        "+{} keys (Page {}/{}): {}",
                        total,
                        page,
                        total_pages,
                        items.join(", ")
                    ))
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
        self.data.remove(&key);
        self.entry_expiries.remove(&key);
        Ok(())
    }

    fn flushall(&mut self) -> Result<()> {
        self.data.clear();
        self.entry_expiries.clear();
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

pub static STORE: LazyLock<RwLock<Store>> = LazyLock::new(|| {
    let store: HashMap<String, CacheEntry> = HashMap::new();
    let store_constructor = Store::new(store);
    RwLock::new(store_constructor)
});

pub async fn start_expiry_worker() {
    let mut tick = tokio::time::interval(Duration::from_millis(100));
    loop {
        tick.tick().await;
        let aggressive = {
            let mut store = STORE.write();
            let ttl_count = store.entry_expiries.len();
            if ttl_count == 0 {
                continue;
            }

            let sample_size = ttl_count.min(20);
            let expired_count: usize = store
                .entry_expiries
                .iter()
                .take(sample_size)
                .filter(|(_, deadline)| Instant::now() >= **deadline)
                .count();

            let ratio = expired_count as f64 / sample_size as f64;
            let do_aggressive = ratio > 0.25;

            let expired: Vec<String> = if do_aggressive {
                store
                    .entry_expiries
                    .iter()
                    .filter(|(_, deadline)| Instant::now() >= **deadline)
                    .map(|(k, _)| k.clone())
                    .collect()
            } else {
                store
                    .entry_expiries
                    .iter()
                    .take(sample_size)
                    .filter(|(_, deadline)| Instant::now() >= **deadline)
                    .map(|(k, _)| k.clone())
                    .collect()
            };

            for key in &expired {
                store.data.remove(key);
                store.entry_expiries.remove(key);
            }
            do_aggressive
        };

        if aggressive {
            tokio::time::sleep(Duration::from_millis(100)).await;
        } else {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}
