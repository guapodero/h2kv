pub mod fs_sync;
pub mod server;

mod storage;
pub use storage::StorageFactory;

mod content_negotiation;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: i32,
    pub storage_dir: std::path::PathBuf,
    pub sync_dir: Option<std::path::PathBuf>,
    pub sync_write: bool,
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
