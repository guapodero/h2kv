use std::path::PathBuf;

use anyhow::{Result, bail};
use auto_args::AutoArgs;

#[derive(Debug, AutoArgs)]
struct Opt {
    storage_dir: PathBuf,
    port: Option<i32>,
}

impl TryFrom<Opt> for http2kv::Config {
    type Error = anyhow::Error;

    fn try_from(value: Opt) -> std::result::Result<Self, Self::Error> {
        if !value.storage_dir.as_path().is_dir() {
            bail!("{:?} is not a directory", &value.storage_dir);
        }

        Ok(Self {
            port: value.port.unwrap_or(5928),
            storage_dir: value.storage_dir,
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::try_init()?;

    let config: http2kv::Config = Opt::from_args().try_into()?;

    http2kv::server::listen(config).await?;

    Ok(())
}
