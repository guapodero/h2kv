use std::{env, fs, io::BufRead, process};

use cargo_metadata::{Metadata, MetadataCommand, Package};
use devx_cmd::{cmd, read, run};
use parse_changelog::Release;

use xtask::{ServerProcess, TlsProxy, prelude::*};

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
        Some("release-stage") => release_stage()?,
        Some("release-push") => release_push()?,
        _ => print_help(),
    }

    Ok(())
}

fn print_help() {
    eprintln!(
        "Tasks:

test                run integration tests
release-stage       verify changelog. update version, docs, dependencies
release-push        push release commit and publish to cargo registry
"
    )
}

fn test_integration() -> Result<(), DynError> {
    let sync_dir = read!("mktemp", "--directory")?;
    let sync_dir = sync_dir.trim();

    let sync_file = format!("{sync_dir}/sync_file.wasm");
    fs::write(&sync_file, b"sync_file contents")?;

    fs::write(format!("{sync_dir}/ignored_read"), b"ignored")?;

    let server = ServerProcess::try_start(9080, sync_dir)?;
    let _proxy = TlsProxy::try_start(8443, 9080)?;

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

        let updated_sync_file = fs::read(sync_file).ok();
        assert_eq!(
            updated_sync_file,
            Some(b"sync_file contents updated".to_vec())
        );

        let new_file = fs::read(format!("{sync_dir}/new/file2.txt")).ok();
        assert_eq!(new_file, Some(b"file2 contents".to_vec()));

        assert!(!fs::exists(format!("{sync_dir}/ignored_write"))?);

        fs::remove_dir_all(sync_dir)?;
    }

    result.map_err(|e| match try_exit_status(&e) {
        Ok(4) => io_error("test failure"),
        Ok(3) => io_error("runtime error"),
        _ => io_error(format!("unexpected error: {e}")),
    })
}

fn release_stage() -> Result<(), DynError> {
    let changes = read!("git", "status", "--porcelain")?;
    let change_lines = changes.lines().collect::<Vec<_>>();
    match change_lines.as_slice() {
        [] => (),
        [changelog] if *changelog == " M CHANGELOG.md" => (),
        _ => {
            eprintln!("{changes}");
            eprintln!("commit existing changes and try again");
            std::process::exit(2);
        }
    }

    let md = MetadataCommand::new().exec()?;
    let changelog_path = md.workspace_root.join("CHANGELOG.md");
    let changelog = fs::read_to_string(changelog_path)?;
    let head_release = parse_changelog::parse(&changelog)?[1].clone();

    let changelog_tag = format!("v{}", head_release.version);
    let changelog_updated = is_error(cmd!("git", "describe", changelog_tag));
    if !changelog_updated {
        summarize_unlogged_commits(&head_release)?;
    }

    let main_package = main_package(&md);
    let mut main_package_toml =
        fs::read_to_string(&main_package.manifest_path)?.parse::<toml_edit::DocumentMut>()?;
    main_package_toml["package"]["version"] = toml_edit::value(head_release.version.to_string());
    fs::write(
        &main_package.manifest_path,
        main_package_toml.to_string().as_bytes(),
    )?;

    docs_cli()?;
    run!("cargo", "update")?;

    Ok(())
}

fn release_push() -> Result<(), DynError> {
    let md = MetadataCommand::new().exec()?;
    let main_package = main_package(&md);

    eprintln!("please paste the token found on https://crates.io/me below");
    let mut token = String::new();
    std::io::stdin().lock().read_line(&mut token)?;
    cmd!("cargo", "login").stdin(token).run()?;
    run!(
        "cargo",
        "publish",
        "--package",
        main_package.name.as_str(),
        "--dry-run",
        "--allow-dirty"
    )?;

    let version = main_package.version.to_string();
    let tag = format!("v{version}");
    run!("git", "commit", "--all", "--message", &tag)?;
    run!(
        "git",
        "tag",
        "--sign",
        "--annotate",
        &tag,
        "--message",
        format!("release {tag}")
    )?;

    run!("cargo", "publish", "--package", main_package.name.as_str())?;
    run!("git", "push", "--follow-tags")?;

    Ok(())
}

fn summarize_unlogged_commits(head_release: &Release) -> Result<(), DynError> {
    let unlogged = read!(
        "git",
        "log",
        format!("v{}..HEAD", head_release.version),
        "--pretty=oneline",
        "--abbrev-commit"
    )?;
    let unlogged = unlogged
        .lines()
        .take_while(|line| !line.ends_with(&format!(" v{}", head_release.version)))
        .collect::<Vec<_>>();

    if !unlogged.is_empty() {
        let mut bump = (false, false);
        let mut bump_commits = vec![];
        for line in unlogged.into_iter() {
            let msg = line.split(' ').nth(1).unwrap();
            if msg.starts_with("feat") {
                bump.0 = true;
                bump_commits.push(line);
            } else if msg.starts_with("fix") {
                bump.1 = true;
                bump_commits.push(line);
            }
        }

        if !bump_commits.is_empty() {
            eprintln!(
                "changelog needs to be updated for {} commits since {}\n{}",
                bump_commits.len(),
                head_release.title,
                bump_commits.join("\n"),
            );

            let mut version = semver::Version::parse(head_release.version)?;
            if bump.0 {
                version.minor += 1;
            } else if bump.1 {
                version.patch += 1;
            }
            eprintln!("next version: {version}");

            std::process::exit(2);
        }
    }

    Ok(())
}

fn main_package(md: &Metadata) -> &Package {
    let root = md.workspace_root.to_string();
    let workspace_name = root.rsplit('/').next().unwrap();
    let packages = md.workspace_packages();
    packages
        .into_iter()
        .find(|p| p.name.as_str() == workspace_name)
        .unwrap_or_else(|| panic!("member with name {workspace_name} should exist"))
}

fn docs_cli() -> Result<(), DynError> {
    let bin_path = ServerProcess::bin_path()?;
    let output = read!(bin_path, "--help")?;
    fs::write("CLI.txt", output)?;
    Ok(())
}
