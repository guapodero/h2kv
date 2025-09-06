mod leveldb;

use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use anyhow::Result;

pub struct StorageFactory;

impl StorageFactory {
    pub fn create(storage_dir: &Path, updates_tx: Sender<PathBuf>) -> impl StorageBackend + use<> {
        leveldb::DatabaseWrapper::new(storage_dir.join("leveldb").as_path(), updates_tx)
    }
}

pub trait StorageBackend: Send + Sync + 'static {
    /// Retrieve the value at `path`.
    fn get<P: AsRef<Path>>(&self, path: P) -> Result<Option<Vec<u8>>>;

    /// Store a value at `path`.
    fn put<P: AsRef<Path>>(&self, path: P, value: &[u8]) -> Result<()>;

    /// Delete the value at `path`, if it exists.
    fn delete<P: AsRef<Path>>(&self, path: P) -> Result<()>;

    /// Execute in an atomic combination of `put` and `delete` operations.
    fn batch_update<K, V, I>(&self, iter: I) -> Result<()>
    where
        K: AsRef<Path>,
        V: AsRef<[u8]>,
        I: IntoIterator<Item = (K, Option<V>)>;
}
