goladder
========

This is a program for running a Go "ladder" competition.
The primary goal here is to have exciting games; finding the "best" player
is secondary.

Current status
--------------

It can be used but has some rough edges and should not be exposed to the
Internet.

Requirements
------------

* Rust (see https://rustup.rs/ and https://www.rust-lang.org/)
  Minimal version: 1.62 (might work with versions a little older)
* libsqlite3 (see "Notes on building" in https://crates.io/crates/rusqlite)
  Minimal version: 3.24.0

Installation
------------

* Compile and run the application using `cargo run --release` followed
  by the database pathname.
  If the database file does not exist yet, it will be created and
  initialized automatically.
  By omitting `--release` one can create a debug build, which compiles much
  faster but runs slower.
* Go to http://127.0.0.1:8080/ in a Web browser (this is currently
  hard-coded).

Deployment
----------

For production use, a release build should be created. The application
binary can be found somewhere under `target/`. All necessary data is part of
this binary, except possibly the sqlite3 library.

The binary can be run on any computer sufficiently similar to the build
machine.

Printing
--------

Various pages omit buttons and background colours when printing (using CSS
media queries).

Updating
--------

The idea is that the competition has "seasons", for example two a year.

During a season, the rules are kept constant and only minor updates to the
application are applied.

When a new season is to be started, the players can be exported to a file
using the old application and old database. Then, the new application
(possibly with larger updates) can be started using a new database, and the
players can be imported.
