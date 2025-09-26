# h2kv

A simple object storage system. Provides an unopinionated HTTP/2 interface for
[LevelDB](https://opensource.googleblog.com/2011/07/leveldb-fast-persistent-key-value-store.html)
based on features described in public RFCs.

[![Crates.io](https://img.shields.io/crates/v/h2kv.svg)](https://crates.io/crates/h2kv)
[![CI](https://github.com/guapodero/h2kv/workflows/CI/badge.svg)](https://github.com/guapodero/h2kv/actions)

## Goals

* Facilitate web application portability. The only semantics involved are that of hash tables, HTTP, and JSON.

## Non-goals

* This is not designed to scale. Some of the technical limitations of LevelDB are discussed in
[this video](https://www.youtube.com/watch?v=V_C-T5S-w8g).

## Features

* The URL path is the storage key. Stored objects are managed using HTTP verbs (HEAD, GET, PUT, DELETE).
* Bulk synchronization of objects with local filesystem tree (optional).
* Content negotiation of file formats by file extension and HTTP headers
([RFC 9110](https://www.rfc-editor.org/rfc/rfc9110.html#section-12.5.1)).
* Exhaustive integration tests.

## Warnings

* LevelDB is known to suffer from data corruption during system crashes. Use a durable file system such as
[ZFS](https://en.wikipedia.org/wiki/ZFS#Summary) to mitigate this problem.
* TLS is [mandatory](https://daniel.haxx.se/blog/2015/03/06/tls-in-http2/) for creating HTTP/2 connections
from a web browser. Use a TLS proxy such as [Hitch](https://hitch-tls.org/) for this use case.

## Status

This project is a work in progress, with additional releases planned in the near future.

## Acknowledgements

This project is possible thanks to the authors of the
[leveldb-rs-binding](https://github.com/rim99/leveldb-rs-binding) crate.

## Installation

### Cargo

* Install the rust toolchain in order to have cargo installed by following
  [this](https://www.rust-lang.org/tools/install) guide.
* run `cargo install h2kv`

## Usage

### Example

```sh
# only file sync storage keys ending with .html
export H2KV_IGNORE="**/* !/**/*.html"
export RUST_LOG=h2kv=warn
h2kv \
  --storage-dir /tmp --port 8080 --sync-dir . \
  --daemon --pidfile /tmp/h2kv.pid --log-filename /var/log/h2kv.log

# re-sync when src directory changes
PID="$(cat /tmp/h2kv.pid)"
watchexec --watch src kill -HUP $PID
```

### CLI
```txt
USAGE:
  h2kv  [--version] [--storage-dir STRING] [--port i32] [--sync-dir STRING] [--sync-write] [--daemon] [--pidfile STRING] [--log-filename STRING]

  [--version]             print the package version and exit
  [--storage-dir STRING]  directory to use for storage engine files
  [--port i32]            listening port for TCP connections, default: 5928
  [--sync-dir STRING]     directory to sync with the database on start and SIGHUP
  [--sync-write]          write to the synchronized directory on exit and SIGHUP
  [--daemon]              fork into background process
  [--pidfile STRING]      PID file, ignored unless --daemon is set
  [--log-filename STRING] file to send log messages, ignored unless --daemon is set


Environment Variables:
H2KV_IGNORE:
    Used with --sync-dir option to filter which files are synchronized.
    Format:
    String of glob patterns separated by spaces or newline characters.
    Comments allowed between '#' and end of line.
    Patterns starting with '!' are treated as exceptions (whitelist).
    Pattern syntax: https://docs.rs/glob/latest/glob/struct.Pattern.html
    NOTE: Syntax is similar to .gitignore but not identical.
    Example: "* !/*.html !/static/**/*"

```

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

See [CONTRIBUTING.md](CONTRIBUTING.md).
