use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use color_eyre::{eyre::eyre, Result};
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

use crate::core::{CacheEntry, CacheValue, STORE};

const SNAPSHOT_PATH: &str = "dump.mmr";

#[derive(Serialize, Deserialize)]
struct SnapshotValue {
    tag: u8,
    data: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct SnapshotEntry {
    value: SnapshotValue,
    ttl_expires_at: Option<f64>,
    created_at_elapsed_secs: f64,
}

#[derive(Serialize, Deserialize)]
struct Snapshot {
    entries: HashMap<String, SnapshotEntry>,
}

fn cache_value_to_snapshot(value: &CacheValue) -> SnapshotValue {
    match value {
        CacheValue::STR(s) => SnapshotValue {
            tag: 0,
            data: vec![s.clone()],
        },
        CacheValue::List(list) => SnapshotValue {
            tag: 1,
            data: list.iter().cloned().collect(),
        },
        CacheValue::SET(set) => SnapshotValue {
            tag: 2,
            data: set.iter().cloned().collect(),
        },
        CacheValue::Map(map) => SnapshotValue {
            tag: 3,
            data: map.iter().flat_map(|(k, v)| vec![k.clone(), v.clone()]).collect(),
        },
    }
}

fn snapshot_to_cache_value(snap: &SnapshotValue) -> CacheValue {
    match snap.tag {
        0 => CacheValue::STR(snap.data.first().cloned().unwrap_or_default()),
        1 => CacheValue::List(VecDeque::from(snap.data.clone())),
        2 => CacheValue::SET(snap.data.iter().cloned().collect()),
        3 => {
            let mut map = HashMap::new();
            for chunk in snap.data.chunks(2) {
                if let [k, v] = chunk {
                    map.insert(k.clone(), v.clone());
                }
            }
            CacheValue::Map(map)
        }
        _ => CacheValue::STR(String::new()),
    }
}

pub fn save_snapshot(store_data: &HashMap<String, CacheEntry>, path: &Path) -> Result<()> {
    let inst_now = Instant::now();
    let sys_now = SystemTime::now();
    let mut entries = HashMap::with_capacity(store_data.len());

    for (key, entry) in store_data {
        let ttl_expires_at = entry.ttl.map(|deadline| {
            let remaining = deadline.saturating_duration_since(inst_now);
            (sys_now + remaining)
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64()
        });

        let elapsed = inst_now.saturating_duration_since(entry.created_at);
        let created_at_elapsed = elapsed.as_secs_f64();

        entries.insert(
            key.clone(),
            SnapshotEntry {
                value: cache_value_to_snapshot(&entry.item),
                ttl_expires_at,
                created_at_elapsed_secs: created_at_elapsed,
            },
        );
    }

    let snapshot = Snapshot { entries };
    let encoded = bincode::serialize(&snapshot)
        .map_err(|e| eyre!("Failed to serialize snapshot: {}", e))?;
    std::fs::write(path, encoded)
        .map_err(|e| eyre!("Failed to write snapshot file: {}", e))?;

    Ok(())
}

pub fn load_snapshot(path: &Path) -> Result<HashMap<String, CacheEntry>> {
    let encoded = std::fs::read(path)
        .map_err(|e| eyre!("Failed to read snapshot file: {}", e))?;
    let snapshot: Snapshot = bincode::deserialize(&encoded)
        .map_err(|e| eyre!("Failed to deserialize snapshot: {}", e))?;

    let now = Instant::now();
    let sys_now = SystemTime::now();
    let mut entries = HashMap::with_capacity(snapshot.entries.len());

    for (key, snap_entry) in snapshot.entries {
        let expired = snap_entry.ttl_expires_at.map_or(false, |unix_secs| {
            UNIX_EPOCH + Duration::from_secs_f64(unix_secs) <= sys_now
        });

        let ttl = if expired {
            None
        } else {
            snap_entry.ttl_expires_at.map(|unix_secs| {
                let expires_at = UNIX_EPOCH + Duration::from_secs_f64(unix_secs);
                let remaining = expires_at.duration_since(sys_now).unwrap_or_default();
                now + remaining
            })
        };

        if expired {
            continue;
        }

        let created_at = if snap_entry.created_at_elapsed_secs > 0.0 {
            now.checked_sub(Duration::from_secs_f64(snap_entry.created_at_elapsed_secs))
                .unwrap_or(now)
        } else {
            now
        };

        entries.insert(
            key,
            CacheEntry {
                item: snapshot_to_cache_value(&snap_entry.value),
                ttl,
                created_at,
            },
        );
    }

    Ok(entries)
}

fn snapshot_path() -> PathBuf {
    std::env::var("SNAPSHOT_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(SNAPSHOT_PATH))
}

fn snapshot_interval() -> Duration {
    let secs = std::env::var("SNAPSHOT_INTERVAL")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(1);
    Duration::from_secs(secs)
}

pub fn try_load_at_startup() {
    let path = snapshot_path();
    if !path.exists() {
        return;
    }

    match load_snapshot(&path) {
        Ok(data) => {
            let mut store = STORE.write();
            store.load_from_snapshot(data);
            println!("[INFO] Loaded snapshot with {} keys", store.data().len());
        }
        Err(e) => {
            eprintln!("[WARN] Failed to load snapshot: {}", e);
        }
    }
}

pub async fn start_snapshot_worker() {
    let interval = snapshot_interval();
    let path = snapshot_path();
    let mut last_count = 0u64;

    loop {
        tokio::time::sleep(interval).await;

        let current_count = STORE.read().change_count();
        if current_count == last_count {
            continue;
        }

        let data = {
            let mut store = STORE.write();
            store.purge_expired();
            store.data().clone()
        };

        if let Err(e) = save_snapshot(&data, &path) {
            eprintln!("[ERROR] Snapshot failed: {}", e);
        } else {
            last_count = current_count;
        }
    }
}
