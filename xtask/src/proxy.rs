use std::fs;

use devx_cmd::{read, run};

use crate::prelude::*;

pub struct TlsProxy {
    proxy_pid: String,
    temp_dir: String,
}

impl TlsProxy {
    pub fn try_start(frontend: u32, backend: u32) -> Result<Self, DynError> {
        let temp_dir = read!("mktemp", "--directory")?;
        let temp_dir = temp_dir.trim();
        let key_file = format!("{temp_dir}/example.com.key");
        let crt_file = format!("{temp_dir}/example.crt");
        let pem_file = format!("{temp_dir}/example.pem");
        let pid_file = format!("{temp_dir}/hitch.pid");

        nix_shell(format!(
            "openssl \
                req -subj '/CN=example.com' \
                -newkey rsa:2048 -sha256 -keyout {key_file} \
                -nodes -x509 -days 365 -out {crt_file} 2> /dev/null \
            && cat {key_file} {crt_file} > {pem_file} \
            && hitch \
                --alpn-protos='h2' \
                --frontend='[*]:{frontend}' \
                --backend='[127.0.0.1]:{backend}' \
                --daemon=on \
                --pidfile={pid_file} \
                {pem_file} 2> /dev/null"
        ))?
        .wait()?;
        let proxy_pid = read!("cat", pid_file)?;

        Ok(Self {
            proxy_pid,
            temp_dir: temp_dir.to_string(),
        })
    }
}

impl Drop for TlsProxy {
    fn drop(&mut self) {
        let _ = run!("kill", &self.proxy_pid);
        let _ = fs::remove_dir_all(&self.temp_dir);
    }
}
