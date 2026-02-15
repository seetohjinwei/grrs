use anyhow::Result;

mod commands;

fn main() -> Result<()> {
    env_logger::init();

    commands::run()
}
