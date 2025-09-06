use std::fs;

use devx_cmd::{read, run};

use crate::{DynError, nix_shell};

pub struct ServerProcess {
    server_pid: String,
    temp_dir: String,
}

impl ServerProcess {
    pub fn try_start(port: u32, sync_dir: &str) -> Result<Self, DynError> {
        assert!(read!("test", "-d", sync_dir).is_ok());

        let bin_path = fs::canonicalize(format!(
            "{}/../target/debug/h2kv",
            env!("CARGO_MANIFEST_DIR")
        ))?
        .to_string_lossy()
        .into_owned();
        assert!(read!("test", "-x", &bin_path).is_ok());

        let temp_dir = read!("mktemp", "--directory")?;
        let temp_dir = temp_dir.trim();
        let pid_file = format!("{temp_dir}/h2kv.pid");

        nix_shell(format!(
            "{bin_path} \
                --port {port} \
                --storage-dir {temp_dir} \
                --sync-dir {sync_dir} \
                --sync-write &! ; \
                echo $! > {pid_file}
            "
        ))?
        .wait()?;
        let server_pid = read!("cat", pid_file)?;

        Ok(Self {
            server_pid: server_pid.trim().to_string(),
            temp_dir: temp_dir.to_string(),
        })
    }
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        let _ = run!("kill", &self.server_pid);
        let _ = fs::remove_dir_all(&self.temp_dir);
    }
}
