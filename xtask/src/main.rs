use std::{env, fs, process};

use devx_cmd::read;

use xtask::{DynError, ServerProcess, TlsProxy, io_error, nix_shell, try_exit_status};

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{}", e);
        process::exit(-1);
    }
}

fn try_main() -> Result<(), DynError> {
    if cfg!(debug_assertions) {
        env_logger::Builder::from_default_env()
            .format_timestamp(None)
            .try_init()?;
    } else {
        env_logger::try_init()?;
    }

    let task = env::args().nth(1);
    match task.as_deref() {
        Some("test") => test_integration()?,
        _ => print_help(),
    }

    Ok(())
}

fn print_help() {
    eprintln!(
        "Tasks:

test            run integration tests
"
    )
}

fn test_integration() -> Result<(), DynError> {
    let sync_dir = read!("mktemp", "--directory")?;
    let sync_dir = sync_dir.trim();
    let sync_file = format!("{sync_dir}/sync_file");
    fs::write(&sync_file, b"sync_file contents")?;

    let server = ServerProcess::try_start(8080, sync_dir)?;
    let _proxy = TlsProxy::try_start(8443, 8080)?;

    let result = nix_shell(
        "hurl \
            --http2 --insecure --variable PORT=8443 \
            --test --jobs 1 \
            tests/*.hurl",
    )?
    .wait();

    if result.is_ok() {
        // stop the process
        drop(server);
        // wait for filesystem sync to finish
        std::thread::sleep(std::time::Duration::from_millis(500));
        let updated_sync_file = String::from_utf8(fs::read(sync_file)?)?;
        assert_eq!(updated_sync_file, "sync_file contents updated");
        fs::remove_dir_all(sync_dir)?;
    }

    if let Err(e) = &result {
        let error = match try_exit_status(e) {
            Ok(4) => io_error("test failure"),
            Ok(3) => io_error("runtime error"),
            _ => io_error(&format!(
                "unexpected error: {}",
                result.err().unwrap().to_string()
            )),
        };
        Err(error)
    } else {
        Ok(())
    }
}
