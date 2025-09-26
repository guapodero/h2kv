use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::{fs, io};

use anyhow::{Context, Result, anyhow};
use walkdir::{DirEntry, WalkDir};

use crate::IgnoreFilter;
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
pub fn store_each_file(
    sync_dir: &Path,
    db: Arc<impl StorageBackend>,
    ignore: &IgnoreFilter,
) -> Result<()> {
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
        if ignore.matches(&storage_key) {
            continue;
        }

        let empty_headers = http::HeaderMap::default();
        let mut negotiated = NegotiatedPath::for_write(storage_key, &empty_headers)?.unwrap();
        if storage_key.extension().is_some()
            && let Err(e) = negotiated.guess_media_type()
        {
            log::warn!("media type guess failed for {negotiated}: {e}");
        }
        let mut extensions = PathExtensions::get_for_path(storage_key, db.clone());

        let content = fs::read(&file_path).with_context(|| format!("read {file_path:?} failed"))?;
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
    ignore: &IgnoreFilter,
) -> Result<()> {
    for storage_key in update_keys {
        if ignore.matches(storage_key) {
            log::warn!("write filter ignored {storage_key:?}");
            continue;
        }

        let relative_path = storage_key.strip_prefix("/").unwrap();
        let mut file_path = sync_dir.join(relative_path);

        // remove the fake file extension that was added for content negotiation
        if matches!(
            file_path.extension().unwrap().to_str(),
            Some(NegotiatedPath::GENERIC_EXT)
        ) {
            file_path.set_extension("");
        }

        match db.get(storage_key)? {
            Some(stored) => {
                let file_directory = file_path.parent().unwrap();
                fs::create_dir_all(file_directory)
                    .with_context(|| format!("create directory {file_directory:?} failed"))?;
                fs::write(&file_path, stored)
                    .with_context(|| format!("write {file_path:?} failed"))?;
            }
            None => fs::remove_file(&file_path).or_else(|e| match e.kind() {
                // storage key was added and then removed
                io::ErrorKind::NotFound => Ok(()),
                _ => Err(anyhow!("remove {file_path:?} failed: {e}")),
            })?,
        }
    }

    Ok(())
}
