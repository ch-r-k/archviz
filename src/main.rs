mod enricher;
mod error;
mod model;
mod parser;
mod pipeline;
mod project;
mod renderer;

use anyhow::Result;
use pipeline::Pipeline;

fn main() -> Result<()> {
    let root = std::env::args().nth(1).expect("usage: archviz <path>");

    let output = Pipeline::builder(root).build().run()?;
    println!("{}", output);

    Ok(())
}
