use std::process;

use devx_cmd::Cmd;

mod server_process;
pub use server_process::*;

mod proxy;
pub use proxy::*;

pub type DynError = Box<dyn std::error::Error>;

pub fn io_error(msg: &str) -> DynError {
    std::io::Error::other(msg).into()
}

pub fn nix_shell<S: AsRef<str>>(script: S) -> Result<devx_cmd::Child, DynError> {
    Cmd::new("nix")
        .arg("--version")
        .log_err(None)
        .spawn_with(process::Stdio::null(), process::Stdio::null())
        .map_err(|e| std::io::Error::other(format!("nix executable not found: {e}")))?;

    let child = Cmd::new("nix")
        .env(
            "NIX_CONFIG",
            "extra-experimental-features = nix-command flakes",
        )
        .args(&[
            "develop",
            "path:tests",
            "--command",
            "sh",
            "-c",
            script.as_ref(),
        ])
        .log_err(Some(log::Level::Debug))
        .spawn()?;

    Ok(child)
}

pub fn try_exit_status(error: &devx_cmd::Error) -> Result<u32, String> {
    let msg = error.to_string();
    let maybe_code: Option<u32> = msg
        .split("exit status:")
        .nth(1)
        .map(|m| {
            let code_str = m.chars().take_while(|&c| c != '\n').collect::<String>();
            code_str.trim().parse().ok()
        })
        .flatten();

    match maybe_code {
        Some(code) => Ok(code),
        None => Err(msg),
    }
}
