use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc::Receiver};

use anyhow::{Result, anyhow, bail};

use crate::{Config, StorageBackend, fs_sync};

/// On success, returns `Ok(None)` to parent and `Ok(Some(resources))` to daemon.
pub fn spawn_daemon<F, L>(config: &Config, lock_resources: F) -> Result<Option<L>>
where
    F: FnOnce() -> Result<L> + 'static,
{
    use daemonize::{Daemonize, Outcome};
    use std::io::{BufRead, Read, pipe};

    let (mut exit_status_reader, mut exit_status_writer) = pipe()?;
    let (exit_message_reader, mut exit_message_writer) = pipe()?;

    let mut daemon_result = |lock_resources_result: Result<L>| -> Result<Option<L>> {
        match lock_resources_result {
            Ok(resources) => {
                exit_status_writer.write_all(&[0])?;
                Ok(Some(resources))
            }
            Err(e) => {
                let msg = format!("failed to spawn daemon. resource lock failure: {e}");
                exit_status_writer.write_all(&[2])?;
                exit_message_writer.write_all([&msg, "\n"].concat().as_bytes())?;
                Err(anyhow!(msg))
            }
        }
    };

    let parent_result = |child_exit_code: i32| -> Result<Option<L>> {
        match child_exit_code {
            0 => {
                let mut status_buf = [0; 1];
                exit_status_reader.read_exact(&mut status_buf)?;
                let exit_status = status_buf[0] as i32;
                if exit_status == 0 {
                    Ok(None)
                } else {
                    let mut msg_buf = String::new();
                    let mut buf_reader = BufReader::new(exit_message_reader);
                    buf_reader.read_line(&mut msg_buf)?;
                    Err(anyhow!(msg_buf))
                }
            }
            code => bail!("abnormal child exit. code: {code}"),
        }
    };

    let mut builder = Daemonize::new().privileged_action(lock_resources);

    if let Some(ref pidfile) = config.pidfile {
        builder = builder.pid_file(pidfile);
    }

    if let Some(ref log_filename) = config.log_filename {
        let stderr = File::create(log_filename)?;
        builder = builder.stderr(stderr);
    }

    match builder.execute() {
        Outcome::Child(result) => match result {
            Ok(child) => daemon_result(child.privileged_action_result),
            Err(e) => Err(anyhow!("spawn failure: {e}")),
        },
        Outcome::Parent(result) => match result {
            Ok(parent) => parent_result(parent.first_child_exit_code),
            Err(e) => Err(anyhow!("spawn failure: {e}")),
        },
    }
}

pub struct FilesystemActions<'a> {
    pub sync_dir: Option<&'a Path>,
    pub sync_write: bool,
    pub updates_rx: &'a Receiver<PathBuf>,
}

impl<'a> FilesystemActions<'a> {
    pub fn do_read(&self, db: Arc<impl StorageBackend>) -> Result<()> {
        if let Some(sync_dir) = self.sync_dir {
            fs_sync::store_each_file(sync_dir, db.clone())?;
            let update_keys = fs_sync::collect_updates(self.updates_rx);
            log::info!(
                "sync-dir: stored {} objects from {sync_dir:?}",
                update_keys.len()
            );
        }
        Ok(())
    }

    pub fn do_write(&self, db: Arc<impl StorageBackend>) -> Result<()> {
        if self.sync_write
            && let Some(sync_dir) = self.sync_dir
        {
            let update_keys = fs_sync::collect_updates(self.updates_rx);
            fs_sync::write_each_key(sync_dir, db, &update_keys)?;
            log::info!(
                "sync-write: wrote {} updates to {sync_dir:?}",
                update_keys.len()
            );
        }
        Ok(())
    }
}
