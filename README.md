goladder
========

This is a program for running a Go "ladder" competition.
The primary goal here is to have exciting games; finding the "best" player
is secondary.

Current status
--------------

This software is not currently recommended for production use.

Requirements
------------

* Rust (see https://rustup.rs/ and https://www.rust-lang.org/)
* libsqlite3 (see "Notes on building" in https://crates.io/crates/rusqlite)
  Minimal version: 3.24.0

Installation
------------

* Create an SQLite3 database.
* Create the necessary tables and types by running `database/schema.sql`
  (for example `sqlite3 goladder.db <database/schema.sql`).
* Compile and run the application using `cargo run --release` followed
  by the database pathname.
  By omitting `--release` one can create a debug build, which compiles much
  faster but runs slower.
* Go to http://127.0.0.1:8080/ in a Web browser (this is currently
  hard-coded).

Deployment
----------

For production use, the application binary can be found somewhere under
`target/`.

It is strongly recommended to place a reverse proxy such as nginx in front
of this. This is useful for TLS, for example. Also, since the application
currently does not do authentication, the reverse proxy will have to handle
that.
