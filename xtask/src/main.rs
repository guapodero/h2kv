use std::{env, fs, io::BufRead, process};

use devx_cmd::{cmd, read};

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
        Some("doc") => docs_cli()?,
        Some("release-stage") => release_stage()?,
        Some("release-commit") => release_commit()?,
        _ => print_help(),
    }

    Ok(())
}

fn print_help() {
    eprintln!(
        "Tasks:

test                run integration tests
doc                 generate CLI.md
release-stage       update docs, dependencies, changelog
release-commit      publish to github and cargo registry
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

    result.map_err(|e| match try_exit_status(&e) {
        Ok(4) => io_error("test failure"),
        Ok(3) => io_error("runtime error"),
        _ => io_error(&format!("unexpected error: {e}")),
    })
}

fn docs_cli() -> Result<(), DynError> {
    let bin_path = ServerProcess::bin_path()?;
    let output = read!(bin_path, "--help")?;
    fs::write("CLI.md", output)?;
    Ok(())
}

fn release_stage() -> Result<(), DynError> {
    let changes = read!("git", "status", "--porcelain")?;
    if !changes.is_empty() {
        eprintln!("{changes}");
        eprintln!("commit existing changes and try again");
        std::process::exit(2);
    }

    docs_cli()?;
    nix_shell("release-plz update")?.wait()?;

    Ok(())
}

fn release_commit() -> Result<(), DynError> {
    eprintln!("please paste the token found on https://crates.io/me below");
    let mut token = String::new();
    std::io::stdin().lock().read_line(&mut token)?;
    cmd!("cargo", "login").stdin(token).run()?;

    nix_shell("release-plz release")?.wait()?;

    Ok(())
}
