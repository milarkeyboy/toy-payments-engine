mod cli;
mod engine;
mod requests;

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Construct the payment engine, wrap it in a CLI, and run.
    let engine = engine::Engine::new();
    let mut cli = cli::Cli::new(engine);
    cli.run()?;

    Ok(())
}
