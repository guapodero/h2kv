# h2kv

A simple object storage system. Provides an unopinionated HTTP/2 interface for
[LevelDB](https://opensource.googleblog.com/2011/07/leveldb-fast-persistent-key-value-store.html)
based on features described in public RFCs.

[![Crates.io](https://img.shields.io/crates/v/h2kv.svg)](https://crates.io/crates/h2kv)
[![Docs.rs](https://docs.rs/h2kv/badge.svg)](https://docs.rs/h2kv)
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

[CLI.txt](CLI.txt)

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
