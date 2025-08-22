use std::path::{Path, PathBuf};

use leveldb::database::Database;
use leveldb::database::serializable::Serializable;
use leveldb::kv::KV;
use leveldb::options::{Options, ReadOptions, WriteOptions};

use anyhow::{Context, Result};

use crate::storage::StorageBackend;

pub struct DatabaseWrapper {
    db: Database<PathKey>,
    write_opts: WriteOptions,
}

impl DatabaseWrapper {
    pub fn new(path: &Path) -> Self {
        let mut opts = Options::new();
        opts.create_if_missing = true;

        let db = Database::open(path, opts)
            .unwrap_or_else(|e| panic!("failed to open db {path:?}: {e:?}"));

        let write_opts = WriteOptions::new();

        Self { db, write_opts }
    }
}

impl StorageBackend for DatabaseWrapper {
    fn get(&self, key: &Path) -> Result<Option<Vec<u8>>> {
        let read_opts = ReadOptions::new();
        self.db
            .get(read_opts, PathKey(key.into()))
            .with_context(|| format!("failed get {}", key.to_str().unwrap()))
    }

    fn put(&self, key: &Path, value: &[u8]) -> Result<()> {
        self.db
            .put(self.write_opts, PathKey(key.into()), value)
            .with_context(|| format!("failed put {}", key.to_str().unwrap()))?;
        Ok(())
    }

    fn delete(&self, key: &Path) -> Result<()> {
        self.db
            .delete(self.write_opts, PathKey(key.into()))
            .with_context(|| format!("failed delete {}", key.to_str().unwrap()))
    }
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PathKey(PathBuf);

impl Serializable for PathKey {
    fn from_u8(key: &[u8]) -> Self {
        let string = String::from_utf8_lossy(key).into_owned();
        let path = PathBuf::from(string);
        Self(path)
    }

    fn as_u8(&self) -> Vec<u8> {
        self.0.as_os_str().to_string_lossy().as_bytes().to_vec()
    }
}
