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

## Planned features

* Naturally discover .gitignore files
* Context flag `-C`
    * Display X leading and trailing context surrounding each match
* Smart context mode (switched on by default)
    * Intended for searching certain strings like `TODO:` that are typically found at the start of a context block
    * Using the prefix before the match, e.g. for `  # TODO:`, `  # ` is the prefix, all continuous lines that share the same prefix are considered as part of the same context
* Highlight the matched substring in a matched line
* Accept pipe as input
    * So that we can do stuff like `grrs --help | grrs context`
* Rename the project to be easier to type. Ideas:
    * gr / gre: grep but faster
    * sg: [s]earch [g]rep -- default keybinding for my nvconf; it's also homerow
* Add subcommands that support related features that would benefit from gitignore:
    * sg grep: what we're doing
    * sg check-ignore: for debugging
    * sg ls: `git ls-files`
    * sg tree: `tree --gitignore`
    * sg wc: `wc` (with ignore functionality)

## Reference links

* https://git-scm.com/docs/gitignore
* https://wasabifan.github.io/combinator-quick-reference/
* https://rust-cli.github.io/book/index.html 
* BurntSushi's ripgrep, ignore, globset crates
    * https://github.com/BurntSushi/ripgrep/blob/master/crates/ignore/src/gitignore.rs
    * https://github.com/BurntSushi/ripgrep/blob/master/crates/globset/src/lib.rs
    * https://github.com/BurntSushi/ripgrep/blob/master/crates/ignore/src/walk.rs
