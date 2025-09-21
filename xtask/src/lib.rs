mod server_process;
pub use server_process::*;

mod proxy;
pub use proxy::*;

pub mod prelude {
    use std::process;

    use devx_cmd::{Cmd, cmd};

    pub type DynError = Box<dyn std::error::Error>;

    pub fn nix_shell<S: AsRef<str>>(script: S) -> Result<devx_cmd::Child, DynError> {
        if is_error(cmd!("nix", "--version")) {
            return Err(io_error("nix executable not found"));
        }

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

    pub fn is_error(cmd: Cmd) -> bool {
        cmd.clone()
            .log_err(None)
            .spawn_with(process::Stdio::null(), process::Stdio::null())
            .unwrap()
            .wait()
            .is_err_and(|e| !matches!(try_exit_status(&e), Ok(0)))
    }

    pub fn try_exit_status(error: &devx_cmd::Error) -> Result<u32, String> {
        let msg = error.to_string();
        let maybe_code: Option<u32> = msg.split("exit status:").nth(1).and_then(|m| {
            let code_str = m
                .chars()
                .take_while(|&c| " 0123456789".contains(c))
                .collect::<String>();
            code_str.trim().parse().ok()
        });

        match maybe_code {
            Some(code) => Ok(code),
            None => Err(msg),
        }
    }

    pub fn io_error<S: AsRef<str>>(msg: S) -> DynError {
        std::io::Error::other(msg.as_ref()).into()
    }
}
