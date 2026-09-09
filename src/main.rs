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
use filter::FilterOptions;
use pipeline::Pipeline;

fn main() -> Result<()> {
    let cli = cli::parse()?;

    // main.rs is the composition root: it copies the CLI's raw string
    // args into the pipeline's `FilterOptions` boundary type. Neither
    // side needs to know about the other's concrete types.
    let filter_options = FilterOptions {
        includes: cli.filter.includes,
        excludes: cli.filter.excludes,
        collapses: cli.filter.collapses,
        collapse_depth: cli.filter.collapse_depth,
    };

    let output = Pipeline::builder(cli.root)
        .with_filter_options(filter_options)?
        .build()
        .run()?;
    println!("{}", output);

    Ok(())
}
