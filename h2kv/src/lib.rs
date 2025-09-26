pub mod runtime;
pub mod server;

mod storage;
pub use storage::{StorageBackend, StorageFactory};

mod content_negotiation;
mod fs_sync;

mod ignore_filter;
pub use ignore_filter::IgnoreFilter;

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: i32,
    pub storage_dir: PathBuf,
    pub sync_dir: Option<PathBuf>,
    pub sync_write: bool,
    pub sync_ignore: IgnoreFilter,
    pub daemon: bool,
    pub pidfile: Option<PathBuf>,
    pub log_filename: Option<PathBuf>,
}

mod util {
    use std::path::{Path, PathBuf};

    /// Returns `path` with all file extensions removed.
    pub fn path_stem(path: &Path) -> PathBuf {
        // temporary shim: see file_prefix https://github.com/rust-lang/rust/issues/86319
        let mut prefix = path;
        while prefix.extension().is_some() {
            prefix = Path::new(prefix.file_stem().unwrap());
        }

        match path.parent() {
            None => prefix.into(),
            Some(parent) => parent.join(prefix),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_path_stem() {
            let path = Path::new("/foo/bar/baz.foy.txt");
            assert_eq!(path_stem(path), PathBuf::from("/foo/bar/baz"));
        }
    }
}
