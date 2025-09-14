use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::{fs, io};

use anyhow::Result;
use walkdir::{DirEntry, WalkDir};

use crate::content_negotiation::{NegotiatedPath, PathExtensions};
use crate::storage::StorageBackend;

pub fn collect_updates(updates_rx: &Receiver<PathBuf>) -> Vec<PathBuf> {
    let mut updates: Vec<PathBuf> = updates_rx.try_iter().collect();
    updates.sort();
    updates.dedup();
    updates
        .into_iter()
        .filter(|k| {
            let ext = k.extension().unwrap().to_str().unwrap();
            ext != PathExtensions::META_EXT
        })
        .collect()
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

        let empty_headers = http::HeaderMap::default();
        let negotiated = NegotiatedPath::for_write(storage_key, &empty_headers)?.unwrap();
        let mut extensions = PathExtensions::get_for_path(storage_key, db.clone());

        let content = fs::read(&file_path)?;
        db.batch_update([
            (negotiated.as_ref(), Some(content)),
            extensions.insert(&negotiated)?,
        ])?;
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
        let mut file_path = sync_dir.join(storage_key.strip_prefix("/").unwrap());

        // remove the fake file extension that was added for content negotiation
        if matches!(
            file_path.extension().unwrap().to_str(),
            Some(NegotiatedPath::GENERIC_EXT)
        ) {
            file_path.set_extension("");
        }

        match db.get(storage_key)? {
            Some(stored) => {
                fs::write(&file_path, stored)?;
            }
            None => fs::remove_file(&file_path).or_else(|e| match e.kind() {
                // storage key was added and then removed
                io::ErrorKind::NotFound => Ok(()),
                _ => Err(e),
            })?,
        }
    }

    Ok(())
}
