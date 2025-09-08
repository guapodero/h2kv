use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use leveldb::batch::{Batch, Writebatch};
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
    pub fn try_new(path: &Path, updates_tx: Sender<PathBuf>) -> Result<Self> {
        let mut opts = Options::new();
        opts.create_if_missing = true;

        let db =
            Database::open(path, opts).with_context(|| format!("failed to open db {path:?}"))?;

        let write_opts = WriteOptions::new();

        Ok(Self {
            db,
            write_opts,
            updates_tx,
        })
    }
}

impl StorageBackend for DatabaseWrapper {
    fn get<P: AsRef<Path>>(&self, path: P) -> Result<Option<Vec<u8>>> {
        let path = path.as_ref();
        let read_opts = ReadOptions::new();
        self.db
            .get(read_opts, PathKey(path.into()))
            .with_context(|| format!("failed get {}", path.to_string_lossy()))
    }

    fn put<P: AsRef<Path>>(&self, path: P, value: &[u8]) -> Result<()> {
        let path = path.as_ref();
        let key = PathKey(path.into());
        self.db
            .put(self.write_opts, key, value)
            .with_context(|| format!("failed put {}", path.to_string_lossy()))?;
        self.updates_tx.send(path.to_owned())?;

        Ok(())
    }

    fn delete<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        let key = PathKey(path.into());
        self.db
            .delete(self.write_opts, key)
            .with_context(|| format!("failed delete {}", path.to_string_lossy()))?;
        self.updates_tx.send(path.to_owned())?;
        Ok(())
    }

    fn batch_update<K, V, I>(&self, iter: I) -> Result<()>
    where
        K: AsRef<Path>,
        V: AsRef<[u8]>,
        I: IntoIterator<Item = (K, Option<V>)>,
    {
        let mut batch = Writebatch::new();
        for (k, v) in iter {
            let k = k.as_ref();
            match v {
                Some(v) => batch.put(PathKey(k.into()), v.as_ref()),
                None => batch.delete(PathKey(k.into())),
            }
            self.updates_tx.send(k.into())?;
        }
        self.db.write(self.write_opts, &batch)?;

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
