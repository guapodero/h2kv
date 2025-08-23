pub mod server;

mod storage;
pub use storage::{StorageFactory, store_each_file, write_each_key};

#[derive(Clone)]
pub struct Config {
    pub port: i32,
    pub storage_dir: std::path::PathBuf,
    pub sync_dir: Option<std::path::PathBuf>,
    pub sync_write: bool,
}
