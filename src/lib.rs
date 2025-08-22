pub mod server;

mod storage;

#[derive(Clone)]
pub struct Config {
    pub port: i32,
    pub storage_dir: std::path::PathBuf,
}
