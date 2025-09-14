use std::path::PathBuf;
use std::sync::{Arc, mpsc};

use anyhow::{Result, anyhow, bail};
use auto_args::AutoArgs;
use tokio::signal::unix::{SignalKind, signal as unix_signal};

#[derive(Debug, AutoArgs)]
struct Opt {
    /// directory to use for storage engine files
    storage_dir: PathBuf,
    /// listening port for TCP connections, default: 5928
    port: Option<i32>,
    /// directory to synchronize with the database and "host" on start and SIGHUP
    sync_dir: Option<PathBuf>,
    /// write to the synchronized directory on exit and SIGHUP
    sync_write: bool,
    /// fork into background process
    daemon: bool,
    /// PID file, ignored unless --daemon is set
    pidfile: Option<PathBuf>,
    /// file to send daemon log messages, ignored unless --daemon is set
    log_filename: Option<PathBuf>,
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

        if value.pidfile.is_some() && !value.daemon {
            log::warn!(
                "'--pidfile {:?}' ignored because '--daemon' is not set",
                value.pidfile.as_ref().unwrap()
            );
        }

        if value.log_filename.is_some() && !value.daemon {
            log::warn!(
                "'--log-filename {:?}' ignored because '--daemon' is not set",
                value.log_filename.as_ref().unwrap()
            );
        }

        Ok(Self {
            port: value.port.unwrap_or(5928),
            storage_dir: value.storage_dir,
            sync_dir: value.sync_dir,
            sync_write: value.sync_write,
            daemon: value.daemon,
            pidfile: value.pidfile,
            log_filename: value.log_filename,
        })
    }
}

fn main() -> Result<()> {
    if cfg!(debug_assertions) {
        env_logger::Builder::from_default_env()
            .format_timestamp(None)
            .try_init()?;
    } else {
        env_logger::try_init()?;
    }

    let config: h2kv::Config = Opt::from_args().try_into()?;

    let (updates_tx, updates_rx) = mpsc::channel::<PathBuf>();

    let storage_dir = config.storage_dir.clone();
    let updates_tx_clone = updates_tx.clone();
    let lock_resources = move || -> Result<_, anyhow::Error> {
        let listener = std::net::TcpListener::bind(format!("127.0.0.1:{}", config.port))?;
        let db = h2kv::StorageFactory::try_create(&storage_dir, updates_tx_clone)?;
        Ok((listener, Arc::new(db)))
    };

    let (listener, db) = if config.daemon {
        match h2kv::runtime::spawn_daemon(&config, lock_resources)? {
            None => {
                log::trace!("daemon spawned. terminating parent");
                return Ok(());
            }
            Some(resources) => {
                log::trace!("daemon process started: {:?}", std::process::id());
                resources
            }
        }
    } else {
        lock_resources().map_err(|e| anyhow!("resource lock failure: {e}"))?
    };

    let files = h2kv::runtime::FilesystemActions {
        sync_dir: config.sync_dir.as_deref(),
        sync_write: config.sync_write,
        updates_rx: &updates_rx,
    };

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            // port and async context are available, so reopen the socket in non-blocking mode
            let addr = listener.local_addr()?;
            drop(listener);
            let listener = tokio::net::TcpListener::bind(addr).await?;

            files.do_read(db.clone())?;

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
                    _ = signal(SignalKind::hangup()), if files.sync_dir.is_some() => {
                        log::info!(
                            "received SIGHUP. synchronizing db and filesystem ({:?})",
                            files.sync_dir.unwrap()
                        );
                        files.do_write(db.clone())?;
                        files.do_read(db.clone())?;
                    }
                    _ = h2kv::server::listen(&listener, db.clone()) => {},
                }
            }

            files.do_write(db)?;

            Ok(())
        })
}

async fn signal(kind: SignalKind) -> std::io::Result<()> {
    unix_signal(kind)?.recv().await;
    Ok(())
}
