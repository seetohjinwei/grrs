use clap::Parser;
use std::io::BufRead;

#[derive(Parser)]
struct Args {
    pattern: String,
    path: std::path::PathBuf,
}

fn grep_file(pattern: String, path: std::path::PathBuf) {
    let f = std::fs::File::open(path).expect("could not open file");
    let reader = std::io::BufReader::new(f);

    for line in reader.lines() {
        let message = line.expect("could not read new line");
        if message.contains(&pattern) {
            println!("{}", message)
        }
    }
}

fn main() {
    let args = Args::parse();

    grep_file(args.pattern, args.path);
}
