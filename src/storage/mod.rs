mod leveldb;

use std::path::Path;

use anyhow::Result;

pub struct StorageFactory;

impl StorageFactory {
    pub fn create(storage_dir: &Path) -> impl StorageBackend + use<> {
        leveldb::DatabaseWrapper::new(storage_dir.join("leveldb").as_path())
    }
}

pub trait StorageBackend: Send + Sync + 'static {
    fn get(&self, key: &Path) -> Result<Option<Vec<u8>>>;

    fn put(&self, key: &Path, value: &[u8]) -> Result<()>;

    fn delete(&self, key: &Path) -> Result<()>;
}
