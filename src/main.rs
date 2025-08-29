use std::path::PathBuf;
use std::sync::{Arc, mpsc};

use anyhow::{Result, bail};
use auto_args::AutoArgs;
use tokio::signal::unix::{SignalKind, signal as unix_signal};

#[derive(Debug, AutoArgs)]
struct Opt {
    /// directory to use for storage engine files
    storage_dir: PathBuf,
    /// listening port for TCP connections
    port: Option<i32>,
    /// directory to synchronize with the database and "host"
    sync_dir: Option<PathBuf>,
    /// whether to write to the synchronized directory on exit
    sync_write: bool,
}

impl TryFrom<Opt> for h2kv::Config {
    type Error = anyhow::Error;

    fn try_from(value: Opt) -> std::result::Result<Self, Self::Error> {
        if !value.storage_dir.as_path().is_dir() {
            bail!("storage-dir {:?} is not a directory", &value.storage_dir);
        }

        match value.sync_dir {
            Some(sync_dir) if !sync_dir.as_path().is_dir() => {
                bail!("sync-dir {:?} is not a directory", &sync_dir);
            }
            _ => (),
        }

        if value.sync_write && value.sync_dir.is_none() {
            bail!("no sync-dir specified for sync-write");
        }

        Ok(Self {
            port: value.port.unwrap_or(5928),
            storage_dir: value.storage_dir,
            sync_dir: value.sync_dir,
            sync_write: value.sync_write,
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::try_init()?;

    let config: h2kv::Config = Opt::from_args().try_into()?;

    let (updates_tx, updates_rx) = mpsc::channel::<PathBuf>();
    let db = Arc::new(h2kv::StorageFactory::create(
        &config.storage_dir,
        updates_tx,
    ));

    if let Some(ref sync_dir) = config.sync_dir {
        h2kv::fs_sync::store_each_file(sync_dir, db.clone())?;
        let update_keys = h2kv::fs_sync::collect_updates(&updates_rx);
        log::info!(
            "sync-dir: stored {} objects from {sync_dir:?}",
            update_keys.len()
        );
    }

    loop {
        tokio::select! {
            biased;
            _ = signal(SignalKind::terminate()) => {
                log::info!("received SIGTERM. exiting");
                break;
            },
            _ = signal(SignalKind::interrupt()) => {
                log::info!("received SIGINT. exiting");
                break;
            },
            _ = h2kv::server::listen(&config, db.clone()) => {},
        }
    }

    if config.sync_write
        && let Some(ref sync_dir) = config.sync_dir
    {
        let update_keys = h2kv::fs_sync::collect_updates(&updates_rx);
        h2kv::fs_sync::write_each_key(sync_dir, db, &update_keys)?;
        log::info!(
            "sync-write: wrote {} updates to {sync_dir:?}",
            update_keys.len()
        );
    }

    Ok(())
}

async fn signal(kind: SignalKind) -> std::io::Result<()> {
    unix_signal(kind)?.recv().await;
    Ok(())
}
