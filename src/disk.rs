use anyhow::Result;
use bytes::Bytes;
use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, SystemTime},
};
use tokio::sync::Mutex;

struct Record {
    size: u64,
    created: SystemTime,
    used: u64,
}
struct Index {
    files: HashMap<String, Record>,
    bytes: u64,
    clock: u64,
}
pub struct DiskCache {
    root: PathBuf,
    max: u64,
    ttl: Duration,
    index: Mutex<Index>,
}

impl DiskCache {
    pub async fn new(root: PathBuf, max: u64, ttl: u64) -> Result<Self> {
        let mut index = Index {
            files: HashMap::new(),
            bytes: 0,
            clock: 0,
        };
        if max > 0 {
            tokio::fs::create_dir_all(&root).await?;
            let mut entries = tokio::fs::read_dir(&root).await?;
            while let Some(entry) = entries.next_entry().await? {
                let name = entry.file_name().to_string_lossy().into_owned();
                if !valid_name(&name) {
                    continue;
                }
                let meta = entry.metadata().await?;
                if !meta.is_file() {
                    continue;
                }
                let created = meta.modified()?;
                if created.elapsed().unwrap_or_default().as_secs() >= ttl {
                    tokio::fs::remove_file(entry.path()).await?;
                } else {
                    index.bytes += meta.len();
                    index.files.insert(
                        name,
                        Record {
                            size: meta.len(),
                            created,
                            used: 0,
                        },
                    );
                }
            }
        }
        let cache = Self {
            root,
            max,
            ttl: Duration::from_secs(ttl),
            index: Mutex::new(index),
        };
        cache.trim(&mut *cache.index.lock().await, 0).await?;
        Ok(cache)
    }

    async fn trim(&self, index: &mut Index, incoming: u64) -> Result<()> {
        while index.bytes.saturating_add(incoming) > self.max || index.files.len() >= 8192 {
            let Some(name) = index
                .files
                .iter()
                .min_by_key(|(_, r)| r.used)
                .map(|(k, _)| k.clone())
            else {
                break;
            };
            match tokio::fs::remove_file(self.root.join(&name)).await {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
            if let Some(r) = index.files.remove(&name) {
                index.bytes -= r.size;
            }
        }
        Ok(())
    }

    pub async fn get(&self, key: &str) -> Option<(Bytes, Duration)> {
        if self.max == 0 {
            return None;
        }
        let mut index = self.index.lock().await;
        index.clock += 1;
        let clock = index.clock;
        let record = index.files.get_mut(key)?;
        if record.created.elapsed().unwrap_or_default() >= self.ttl {
            let _ = tokio::fs::remove_file(self.root.join(key)).await;
            let r = index.files.remove(key)?;
            index.bytes -= r.size;
            return None;
        }
        record.used = clock;
        let remaining = self
            .ttl
            .saturating_sub(record.created.elapsed().unwrap_or_default());
        match tokio::fs::read(self.root.join(key)).await {
            Ok(data) => Some((data.into(), remaining)),
            Err(_) => {
                if let Some(r) = index.files.remove(key) {
                    index.bytes -= r.size;
                }
                None
            }
        }
    }

    pub async fn put(&self, key: &str, bytes: &Bytes) -> Result<()> {
        if self.max == 0 || bytes.len() as u64 > self.max {
            return Ok(());
        }
        let mut index = self.index.lock().await;
        if let Some(r) = index.files.remove(key) {
            index.bytes -= r.size;
        }
        self.trim(&mut index, bytes.len() as u64).await?;
        let tmp = self.root.join(format!("{key}.tmp"));
        tokio::fs::write(&tmp, bytes).await?;
        if let Err(e) = tokio::fs::rename(&tmp, self.root.join(key)).await {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err(e.into());
        }
        index.clock += 1;
        let used = index.clock;
        index.files.insert(
            key.into(),
            Record {
                size: bytes.len() as u64,
                created: SystemTime::now(),
                used,
            },
        );
        index.bytes += bytes.len() as u64;
        Ok(())
    }
}

fn valid_name(name: &str) -> bool {
    name.len() == 64 && name.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn persists_evicts_and_expires() {
        let root = std::env::temp_dir().join(format!("pixerve-cache-{}", std::process::id()));
        let a = "a".repeat(64);
        let b = "b".repeat(64);
        let cache = DiskCache::new(root.clone(), 5, 1).await.unwrap();
        cache.put(&a, &Bytes::from_static(b"aaa")).await.unwrap();
        assert_eq!(cache.get(&a).await.unwrap().0, "aaa");
        cache.put(&b, &Bytes::from_static(b"bbb")).await.unwrap();
        assert!(cache.get(&a).await.is_none());
        drop(cache);
        let cache = DiskCache::new(root.clone(), 5, 1).await.unwrap();
        assert_eq!(cache.get(&b).await.unwrap().0, "bbb");
        tokio::time::sleep(Duration::from_millis(1100)).await;
        assert!(cache.get(&b).await.is_none());
        // Only remove the two explicitly named files and the now-empty test dir.
        let _ = tokio::fs::remove_file(root.join(a)).await;
        let _ = tokio::fs::remove_file(root.join(b)).await;
        tokio::fs::remove_dir(root).await.unwrap();
    }
}
