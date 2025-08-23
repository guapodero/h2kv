mod leveldb;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::Sender;

use anyhow::Result;
use walkdir::{DirEntry, WalkDir};

pub struct StorageFactory;

impl StorageFactory {
    pub fn create(storage_dir: &Path, updates_tx: Sender<PathBuf>) -> impl StorageBackend + use<> {
        leveldb::DatabaseWrapper::new(storage_dir.join("leveldb").as_path(), updates_tx)
    }
}

pub trait StorageBackend: Send + Sync + 'static {
    fn get<P: AsRef<Path>>(&self, key: P) -> Result<Option<Vec<u8>>>;

    fn put<P: AsRef<Path>>(&self, key: P, value: &[u8]) -> Result<()>;

    fn delete<P: AsRef<Path>>(&self, key: P) -> Result<()>;
}

/// Each file found in `sync_dir` will be stored as an object in the database.
pub fn store_each_file(sync_dir: &Path, db: Arc<impl StorageBackend>) -> Result<()> {
    let is_hidden = |entry: &DirEntry| -> bool {
        entry
            .file_name()
            .to_str()
            .map(|s| s.starts_with("."))
            .unwrap_or(false)
    };

    for entry in WalkDir::new(sync_dir)
        .into_iter()
        .filter_map(|r| r.ok().filter(|e| !is_hidden(e) && !e.file_type().is_dir()))
    {
        let file_path = entry.into_path();
        let relative_path = pathdiff::diff_paths(&file_path, sync_dir).unwrap();
        let storage_key = Path::new("/").join(relative_path);
        let storage_key = storage_key.as_path();

        let content = std::fs::read(&file_path)?;
        db.put(storage_key, content.as_slice())?;
        log::trace!("stored {file_path:?}");
    }

    Ok(())
}

/// The state of each object from `update_keys` will be written to `sync_dir`.
pub fn write_each_key(
    sync_dir: &Path,
    db: Arc<impl StorageBackend>,
    update_keys: &Vec<PathBuf>,
) -> Result<()> {
    for storage_key in update_keys {
        let file_path = sync_dir.join(storage_key.to_string_lossy().trim_start_matches("/"));
        match db.get(&storage_key)? {
            Some(stored) => {
                std::fs::write(&file_path, stored)?;
            }
            None => {
                std::fs::remove_file(&file_path)?;
            }
        }
    }

    Ok(())
}
