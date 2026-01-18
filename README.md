# grrs - grep, but built in Rust

Follows the tutorial from: https://rust-cli.github.io/book/index.html 

## Features

`grep` defaults are not very ergonomic by modern standards, so grrs steals some ideas from other tools like ripgrep.

* Supports recursive by default
    * Passing a directory will naturally search all files in that directory
    * Use `--depth` to control the max depth
* Shows line numbers by default

```sh
RUST_LOG=debug cargo run -- pattern src/main.rs
```
