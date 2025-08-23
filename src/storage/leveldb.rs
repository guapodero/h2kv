use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use leveldb::database::Database;
use leveldb::database::serializable::Serializable;
use leveldb::kv::KV;
use leveldb::options::{Options, ReadOptions, WriteOptions};

use anyhow::{Context, Result};

use crate::storage::StorageBackend;

pub struct DatabaseWrapper {
    db: Database<PathKey>,
    write_opts: WriteOptions,
    updates_tx: Sender<PathBuf>,
}

impl DatabaseWrapper {
    pub fn new(path: &Path, updates_tx: Sender<PathBuf>) -> Self {
        let mut opts = Options::new();
        opts.create_if_missing = true;

        let db = Database::open(path, opts)
            .unwrap_or_else(|e| panic!("failed to open db {path:?}: {e:?}"));

        let write_opts = WriteOptions::new();

        Self {
            db,
            write_opts,
            updates_tx,
        }
    }
}

impl StorageBackend for DatabaseWrapper {
    fn get<P: AsRef<Path>>(&self, key: P) -> Result<Option<Vec<u8>>> {
        let read_opts = ReadOptions::new();
        let path = key.as_ref();
        self.db
            .get(read_opts, PathKey(path.into()))
            .with_context(|| format!("failed get {}", path.to_string_lossy()))
    }

    fn put<P: AsRef<Path>>(&self, key: P, value: &[u8]) -> Result<()> {
        let path = key.as_ref();
        self.db
            .put(self.write_opts, PathKey(path.into()), value)
            .with_context(|| format!("failed put {}", path.to_string_lossy()))?;
        self.updates_tx.send(path.to_owned())?;
        Ok(())
    }

    fn delete<P: AsRef<Path>>(&self, key: P) -> Result<()> {
        let path = key.as_ref();
        self.db
            .delete(self.write_opts, PathKey(path.into()))
            .with_context(|| format!("failed delete {}", path.to_string_lossy()))?;
        self.updates_tx.send(path.to_owned())?;
        Ok(())
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
