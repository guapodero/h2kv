use std::fs;

use devx_cmd::{Cmd, cmd, read, run};

use crate::prelude::*;

pub struct ServerProcess {
    server_pid: String,
    temp_dir: String,
}

impl ServerProcess {
    pub fn try_start(port: u32, sync_dir: &str) -> Result<Self, DynError> {
        run!("test", "-d", sync_dir)?;

        let temp_dir = read!("mktemp", "--directory")?;
        let temp_dir = temp_dir.trim();
        let pid_file = format!("{temp_dir}/h2kv.pid");
        let bin_path = Self::bin_path()?;

        Cmd::new(bin_path)
            .args(&[
                "--port",
                port.to_string().as_str(),
                "--storage-dir",
                temp_dir,
                "--sync-dir",
                sync_dir,
                "--sync-write",
                "--daemon",
                "--pidfile",
                &pid_file,
                "--log-filename",
                "/dev/fd/2",
            ])
            .log_err(Some(log::Level::Debug))
            .spawn()?
            .wait()?;
        let server_pid = read!("cat", pid_file)?.trim().to_string();

        // wait for server to start
        std::thread::sleep(std::time::Duration::from_millis(500));
        let process_inactive = is_error(cmd!("ps", "--quick-pid", &server_pid));
        if process_inactive {
            return Err(io_error(format!("process {server_pid} failed to start")));
        }

        Ok(Self {
            server_pid,
            temp_dir: temp_dir.to_string(),
        })
    }

    pub fn bin_path() -> Result<String, DynError> {
        let bin_path = fs::canonicalize(format!(
            "{}/../target/debug/h2kv",
            env!("CARGO_MANIFEST_DIR")
        ))?
        .to_string_lossy()
        .into_owned();
        run!("test", "-x", &bin_path)?;
        Ok(bin_path)
    }
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        let _ = run!("kill", &self.server_pid);
        let _ = fs::remove_dir_all(&self.temp_dir);
    }
}
