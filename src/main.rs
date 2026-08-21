mod cli;
mod enricher;
mod error;
mod filter;
mod model;
mod parser;
mod pipeline;
mod project;
mod renderer;

use anyhow::Result;
use pipeline::Pipeline;

fn main() -> Result<()> {
    let cli = cli::parse()?;

    let output = Pipeline::builder(cli.root)
        .with_module_filter(cli.filter)
        .build()
        .run()?;
    println!("{}", output);

    Ok(())
}
