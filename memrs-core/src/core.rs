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

#[derive(Debug, Clone, PartialEq)]
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
    change_count: u64,
}

impl Store {
    fn new(store: HashMap<String, CacheEntry>) -> Self {
        Self {
            data: store,
            entry_expiries: HashMap::new(),
            change_count: 0,
        }
    }

    pub fn new_empty() -> Self {
        Self {
            data: HashMap::new(),
            entry_expiries: HashMap::new(),
            change_count: 0,
        }
    }

    pub fn change_count(&self) -> u64 {
        self.change_count
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

    pub fn purge_expired(&mut self) {
        let expired: Vec<String> = self
            .entry_expiries
            .iter()
            .filter(|(_, deadline)| Instant::now() >= **deadline)
            .map(|(k, _)| k.clone())
            .collect();
        for key in expired {
            self.data.remove(&key);
            self.entry_expiries.remove(&key);
        }
    }

    pub fn data(&self) -> &HashMap<String, CacheEntry> {
        &self.data
    }

    pub fn load_from_snapshot(&mut self, data: HashMap<String, CacheEntry>) {
        self.data = data;
        self.entry_expiries.clear();
        for (key, entry) in &self.data {
            if let Some(deadline) = entry.ttl {
                self.entry_expiries.insert(key.clone(), deadline);
            }
        }
        self.change_count = 0;
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
                    self.change_count += 1;
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
        self.change_count += 1;
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
        self.change_count += 1;
        Ok(())
    }

    fn flushall(&mut self) -> Result<()> {
        self.data.clear();
        self.entry_expiries.clear();
        self.change_count += 1;
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
                self.change_count += 1;
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
                self.change_count += 1;
                Ok(())
            }
            _ => Err(eyre!("Key is not a list")),
        }
    }

    fn lpop(&mut self, key: String) -> Result<String> {
        let entry = self.data.get_mut(&key);
        match entry {
            Some(entry) => match &mut entry.item {
                CacheValue::List(list) => {
                    let val = list.pop_front().ok_or_else(|| eyre!("List is empty"));
                    if val.is_ok() {
                        self.change_count += 1;
                    }
                    val
                }
                _ => Err(eyre!("Key is not a list")),
            },
            None => Err(eyre!("No records have been found")),
        }
    }

    fn rpop(&mut self, key: String) -> Result<String> {
        let entry = self.data.get_mut(&key);
        match entry {
            Some(entry) => match &mut entry.item {
                CacheValue::List(list) => {
                    let val = list.pop_back().ok_or_else(|| eyre!("List is empty"));
                    if val.is_ok() {
                        self.change_count += 1;
                    }
                    val
                }
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
                self.change_count += 1;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_store() -> Store {
        Store::new_empty()
    }

    fn str_entry(v: &str) -> CacheEntry {
        CacheEntry {
            item: CacheValue::STR(v.into()),
            ttl: None,
            created_at: Instant::now(),
        }
    }

    #[test]
    fn set_and_get() {
        let mut store = make_store();
        store.set("k".into(), str_entry("v")).unwrap();
        let got = store.get("k".into()).unwrap();
        assert_eq!(got.item, CacheValue::STR("v".into()));
    }

    #[test]
    fn get_nonexistent_returns_err() {
        let store = make_store();
        assert!(store.get("missing".into()).is_err());
    }

    #[test]
    fn exists_returns_ok_for_existing_key() {
        let mut store = make_store();
        store.set("k".into(), str_entry("v")).unwrap();
        assert!(store.exists("k".into()).is_ok());
    }

    #[test]
    fn exists_returns_err_for_missing_key() {
        let store = make_store();
        assert!(store.exists("missing".into()).is_err());
    }

    #[test]
    fn del_removes_key() {
        let mut store = make_store();
        store.set("k".into(), str_entry("v")).unwrap();
        store.del("k".into()).unwrap();
        assert!(store.get("k".into()).is_err());
    }

    #[test]
    fn flushall_clears_all() {
        let mut store = make_store();
        store.set("k1".into(), str_entry("v1")).unwrap();
        store.set("k2".into(), str_entry("v2")).unwrap();
        store.flushall().unwrap();
        assert!(store.get("k1".into()).is_err());
        assert!(store.get("k2".into()).is_err());
    }

    #[test]
    fn change_count_increments_on_write() {
        let mut store = make_store();
        assert_eq!(store.change_count(), 0);
        store.set("k".into(), str_entry("v")).unwrap();
        assert_eq!(store.change_count(), 1);
        store.del("k".into()).unwrap();
        assert_eq!(store.change_count(), 2);
    }

    #[test]
    fn hset_and_hget() {
        let mut store = make_store();
        store.hset("h".into(), "f1".into(), "v1".into()).unwrap();
        let val = store.hget("h".into(), "f1".into()).unwrap();
        assert_eq!(val, "v1");
    }

    #[test]
    fn hget_nonexistent_field_returns_err() {
        let mut store = make_store();
        store.hset("h".into(), "f1".into(), "v1".into()).unwrap();
        assert!(store.hget("h".into(), "missing".into()).is_err());
    }

    #[test]
    fn hset_type_mismatch_returns_err() {
        let mut store = make_store();
        store.set("k".into(), str_entry("v")).unwrap();
        assert!(store.hset("k".into(), "f".into(), "v".into()).is_err());
    }

    #[test]
    fn lpush_and_lpop() {
        let mut store = make_store();
        store.lpush("l".into(), "a".into()).unwrap();
        store.lpush("l".into(), "b".into()).unwrap();
        assert_eq!(store.lpop("l".into()).unwrap(), "b");
        assert_eq!(store.lpop("l".into()).unwrap(), "a");
    }

    #[test]
    fn rpush_and_rpop() {
        let mut store = make_store();
        store.rpush("l".into(), "a".into()).unwrap();
        store.rpush("l".into(), "b".into()).unwrap();
        assert_eq!(store.rpop("l".into()).unwrap(), "b");
        assert_eq!(store.rpop("l".into()).unwrap(), "a");
    }

    #[test]
    fn lpop_empty_list_returns_err() {
        let mut store = make_store();
        store.lpush("l".into(), "a".into()).unwrap();
        store.lpop("l".into()).unwrap();
        assert!(store.lpop("l".into()).is_err());
    }

    #[test]
    fn rpop_empty_list_returns_err() {
        let mut store = make_store();
        store.rpush("l".into(), "a".into()).unwrap();
        store.rpop("l".into()).unwrap();
        assert!(store.rpop("l".into()).is_err());
    }

    #[test]
    fn list_type_mismatch_returns_err() {
        let mut store = make_store();
        store.set("k".into(), str_entry("v")).unwrap();
        assert!(store.lpush("k".into(), "x".into()).is_err());
        assert!(store.lpop("k".into()).is_err());
    }

    #[test]
    fn hash_type_mismatch_returns_err() {
        let mut store = make_store();
        store.lpush("k".into(), "x".into()).unwrap();
        assert!(store.hget("k".into(), "f".into()).is_err());
    }

    #[test]
    fn expire_sets_ttl_on_existing_key() {
        let mut store = make_store();
        store.set("k".into(), str_entry("v")).unwrap();
        store
            .execute(ReplCommands::EXPIRE("k".into(), 9999))
            .unwrap();
        assert!(store.entry_expiries.contains_key("k"));
    }

    #[test]
    fn expire_nonexistent_key_returns_err() {
        let mut store = make_store();
        let result = store.execute(ReplCommands::EXPIRE("missing".into(), 10));
        assert!(result.is_err());
    }

    #[test]
    fn check_expiry_removes_expired_entry() {
        let mut store = make_store();
        let past = Instant::now() - Duration::from_secs(10);
        store
            .data
            .insert("k".into(), str_entry("v"));
        store.data.get_mut("k").unwrap().ttl = Some(past);
        store.entry_expiries.insert("k".into(), past);
        store.check_expiry("k");
        assert!(!store.data.contains_key("k"));
    }

    #[test]
    fn check_expiry_does_not_remove_fresh_entry() {
        let mut store = make_store();
        store.set("k".into(), str_entry("v")).unwrap();
        store.check_expiry("k");
        assert!(store.data.contains_key("k"));
    }

    #[test]
    fn purge_expired_removes_all_expired() {
        let mut store = make_store();
        let past = Instant::now() - Duration::from_secs(10);
        let future = Instant::now() + Duration::from_secs(9999);
        store.data.insert("expired".into(), CacheEntry {
            item: CacheValue::STR("x".into()),
            ttl: Some(past),
            created_at: Instant::now(),
        });
        store.entry_expiries.insert("expired".into(), past);
        store.data.insert("fresh".into(), CacheEntry {
            item: CacheValue::STR("y".into()),
            ttl: Some(future),
            created_at: Instant::now(),
        });
        store.entry_expiries.insert("fresh".into(), future);
        store.purge_expired();
        assert!(!store.data.contains_key("expired"));
        assert!(store.data.contains_key("fresh"));
    }

    #[test]
    fn execute_ping() {
        let mut store = make_store();
        let resp = store.execute(ReplCommands::PING).unwrap();
        assert_eq!(resp, "+PONG");
    }

    #[test]
    fn execute_set_and_get() {
        let mut store = make_store();
        store
            .execute(ReplCommands::SET("k".into(), "v".into(), None))
            .unwrap();
        let resp = store.execute(ReplCommands::GET("k".into())).unwrap();
        assert_eq!(resp, "v");
    }

    #[test]
    fn execute_set_with_ttl_then_get() {
        let mut store = make_store();
        store
            .execute(ReplCommands::SET("k".into(), "v".into(), Some(9999)))
            .unwrap();
        let resp = store.execute(ReplCommands::GET("k".into())).unwrap();
        assert_eq!(resp, "v");
        assert!(store.entry_expiries.contains_key("k"));
    }

    #[test]
    fn execute_get_nonexistent() {
        let mut store = make_store();
        let resp = store.execute(ReplCommands::GET("missing".into()));
        assert!(resp.is_err());
    }

    #[test]
    fn execute_del() {
        let mut store = make_store();
        store
            .execute(ReplCommands::SET("k".into(), "v".into(), None))
            .unwrap();
        store.execute(ReplCommands::DEL("k".into())).unwrap();
        assert!(store.execute(ReplCommands::GET("k".into())).is_err());
    }

    #[test]
    fn execute_exists() {
        let mut store = make_store();
        store
            .execute(ReplCommands::SET("k".into(), "v".into(), None))
            .unwrap();
        store.execute(ReplCommands::EXISTS("k".into())).unwrap();
        assert!(store
            .execute(ReplCommands::EXISTS("missing".into()))
            .is_err());
    }

    #[test]
    fn execute_flushall() {
        let mut store = make_store();
        store
            .execute(ReplCommands::SET("k".into(), "v".into(), None))
            .unwrap();
        store.execute(ReplCommands::FLUSHALL).unwrap();
        assert!(store.execute(ReplCommands::GET("k".into())).is_err());
    }

    #[test]
    fn execute_hset_hget() {
        let mut store = make_store();
        store
            .execute(ReplCommands::HSET("h".into(), "f".into(), "v".into()))
            .unwrap();
        let val = store
            .execute(ReplCommands::HGET("h".into(), "f".into()))
            .unwrap();
        assert_eq!(val, "v");
    }

    #[test]
    fn execute_lpush_lpop() {
        let mut store = make_store();
        store
            .execute(ReplCommands::LPUSH("l".into(), "a".into()))
            .unwrap();
        store
            .execute(ReplCommands::LPUSH("l".into(), "b".into()))
            .unwrap();
        let val = store.execute(ReplCommands::LPOP("l".into())).unwrap();
        assert_eq!(val, String::from("b"));
    }

    #[test]
    fn execute_rpush_rpop() {
        let mut store = make_store();
        store
            .execute(ReplCommands::RPUSH("l".into(), "a".into()))
            .unwrap();
        store
            .execute(ReplCommands::RPUSH("l".into(), "b".into()))
            .unwrap();
        let val = store.execute(ReplCommands::RPOP("l".into())).unwrap();
        assert_eq!(val, String::from("b"));
    }

    #[test]
    fn execute_listall_empty() {
        let mut store = make_store();
        let resp = store
            .execute(ReplCommands::LISTALL(None))
            .unwrap();
        assert!(resp.contains("0 keys"));
    }

    #[test]
    fn execute_listall_with_entries() {
        let mut store = make_store();
        store
            .execute(ReplCommands::SET("a".into(), "1".into(), None))
            .unwrap();
        store
            .execute(ReplCommands::SET("b".into(), "2".into(), None))
            .unwrap();
        let resp = store.execute(ReplCommands::LISTALL(None)).unwrap();
        assert!(resp.contains("2 keys"));
        assert!(resp.contains("a") || resp.contains("b"));
    }

    #[test]
    fn execute_listall_pagination() {
        let mut store = make_store();
        for i in 0..15 {
            let k = format!("k{}", i);
            store
                .execute(ReplCommands::SET(k, "v".into(), None))
                .unwrap();
        }
        let page1 = store.execute(ReplCommands::LISTALL(Some(1))).unwrap();
        let page2 = store.execute(ReplCommands::LISTALL(Some(2))).unwrap();
        assert!(page1.contains("Page 1"));
        assert!(page2.contains("Page 2"));
    }

    #[test]
    fn load_from_snapshot_resets_change_count() {
        let mut store = make_store();
        store.set("k".into(), str_entry("v")).unwrap();
        assert_eq!(store.change_count(), 1);
        let data = store.data().clone();
        store.load_from_snapshot(data);
        assert_eq!(store.change_count(), 0);
        assert!(store.data.contains_key("k"));
    }
}

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
