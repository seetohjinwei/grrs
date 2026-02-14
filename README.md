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
# To print out `println!` while the program is executing
cargo test test_thread_pool_simple -- --nocapture

cargo build
target/debug/grrs --help

cargo build --release
target/release/grrs --help
```

## Implementation Details

TODO: Fill this section with interesting implementation details
Also, it might be time to revive my blog!

### Handling GitIgnore

Even though there is a well-known and widely-used [`ignore` crate](https://crates.io/crates/ignore/0.4.25), I decided to write my own ignore crate to learn string parsing and file handling in Rust! However, I did not try to fully implement it from scratch. Instead, I chose to translate the ignore pattern to regex. This way, I can leverage the `regex` crate to do the bulk of the work. When I feel confident enough, I do want to re-write this crate to **not** depend on regex! It could allow the tool to gather more debug information, or at least in a less hacky way.

It was a great exercise because the syntax is not too complicated, [the specifications](https://git-scm.com/docs/gitignore) are only about two pages long (with a bunch of examples)! It forced me to better understand some Rust basics like `&str` vs String ([check out this great explanation](https://stackoverflow.com/questions/24158114/what-are-the-differences-between-rusts-string-and-str)). The implementation was not that complicated and it only ended up being about 400 lines of code! I am sure that it is not 100% compliant to the specification, but I am confident that it is reasonably correct! In particular, I am proud of coming up with the idea to split the pattern into parts and parsing it part-by-part. This greatly simplified the handling of the double asterisk rule because the logic is different for leading and trailing parts.

As of writing, the `walk` function is implemented as a recursive DFS that returns a vector of matching file paths. I chose to use DFS because a gitignore is effective on a file tree. This maps nicely into how DFS works: find a gitignore, parse it and use it for the rest of the file tree! Recursion was chosen to have an easy way to clean up the gitignore as a post-DFS step.

```rs
// If gitignore exists in this directory, add it to the stack
let gitignore = GitIgnore::from_dir(&path).unwrap_or(None);
let has_gitignore = gitignore.is_some();
if let Some(gitignore) = gitignore {
    walker.gitignore_stack.push(gitignore);
}

for entry in std::fs::read_dir(path)? {
    let entry = entry?;
    let child = entry.path();
    walk_dfs(walker, child, current_depth + 1)?;
}

// Clean it up from the stack
if has_gitignore {
    let _ = walker.gitignore_stack.pop();
}
```

However, I have plans to parallelize the matching functionality. After that's implemented, it would make sense for `walk` to be an iterator. This way, we wouldn't have to wait for `walk` to finish walking a (potentially massive) file tree before we start matching! If we want it to be an iterator, we cannot use recursion!! There's no way for us to freeze the iterator. Thus, (I think) the best way forward is to convert it to be an iterative DFS and find another way to handle the post-DFS cleanup. Maybe something as simple as "while path is not a descendant of `gitignore_stack.peek().root_path`: `gitignore_stack.pop()`" would work!

## Planned features

* Improve edge cases for walk fn
    * `sg pattern target/` where `target/` is gitignore'd in `./`, but because the walk fn does not look at `./`, it never discovers `./.gitignore`.
        * Simple solution: Always check for `.gitignore` in current working directory
        * More robust solution: If the path is relative to current working directory (does not necessarily mean that the provided path is a relative path -- it can still be an absolute path, but relative to cwd), walk to it from current working directory, picking up all `.gitignore` along the way
        * Better solution: Walk backwards from the target to `/`. Skip any directories that we cannot read, but continue processing them
* Sprinkle more logging in various places
* Accept a `--verbose` flag
    * Switches on debug logging
    * ...?
* Context flag `-C`
    * Display X leading and trailing context surrounding each match
* Smart context mode (switched on by default)
    * Intended for searching certain strings like `TODO:` that are typically found at the start of a context block
    * Using the prefix before the match, e.g. for `  # TODO:`, `  # ` is the prefix, all continuous lines that share the same prefix are considered as part of the same context
* Highlight / .colorize the matched substring in a matched line
* Accept pipe as input
    * So that we can do stuff like `grrs --help | grrs context`
* Rename the project to be easier to type. Ideas:
    * gr / gre: grep but faster
    * sg: [s]earch [g]rep -- default keybinding for my nvconf; it's also homerow
    * ss: because we want to support subcommands! Then, we can alias `sg` to `ss grep`
* Add subcommands that support related features that would benefit from gitignore:
    * ss grep: what we're doing
    * ss check-ignore <filepath>: for debugging
    * ss ls: `git ls-files`
    * ss tree: `tree --gitignore`
    * ss wc: `wc` (with ignore functionality)
    * ss snitch: https://github.com/tsoding/snitch
        * Specifically the urgency levels and the smart context

## Reference links

* https://git-scm.com/docs/gitignore
* https://wasabifan.github.io/combinator-quick-reference/
* https://stackoverflow.com/questions/24158114/what-are-the-differences-between-rusts-string-and-str
* https://rust-cli.github.io/book/index.html 
* BurntSushi's ripgrep, ignore, globset crates
    * https://github.com/BurntSushi/ripgrep/blob/master/crates/ignore/src/gitignore.rs
    * https://github.com/BurntSushi/ripgrep/blob/master/crates/globset/src/lib.rs
    * https://github.com/BurntSushi/ripgrep/blob/master/crates/ignore/src/walk.rs
