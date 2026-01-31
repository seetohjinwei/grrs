# grrs - grep, but built in Rust

Wrote this to learn Rust and other related stuff (i.e. gitignore, file walking)

## Features

`grep` defaults are not very ergonomic by modern standards, so grrs steals some ideas from other tools like ripgrep.

* Supports recursive by default
    * Passing a directory will naturally search all files in that directory
    * Use `--depth` to control the max depth
* Shows line numbers by default

```sh
cargo run -- --help

RUST_LOG=debug cargo run -- pattern src/main.rs
RUST_LOG=debug cargo run -- pattern

cargo fmt
cargo test

cargo build
target/debug/grrs --help

cargo build --release
target/release/grrs --help
```

## Reference links

* https://git-scm.com/docs/gitignore
* https://wasabifan.github.io/combinator-quick-reference/
* https://rust-cli.github.io/book/index.html 
* BurntSushi's ripgrep, ignore, globset crates
    * https://github.com/BurntSushi/ripgrep/blob/master/crates/ignore/src/gitignore.rs
    * https://github.com/BurntSushi/ripgrep/blob/master/crates/globset/src/lib.rs
    * https://github.com/BurntSushi/ripgrep/blob/master/crates/ignore/src/walk.rs
