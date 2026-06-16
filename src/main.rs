mod model;
mod parser;
mod project;
mod renderer;

use anyhow::Result;

use model::Graph;
use parser::AstParser;
use parser::visitor::GraphVisitor;
use project::ProjectLoader;
use renderer::plantuml::PlantUmlRenderer;

fn main() -> Result<()> {
    let root = std::env::args().nth(1).expect("usage: archviz <path>");

    let loader = ProjectLoader::new(root);
    let files = loader.load()?;

    let mut graph = Graph::default();

    for file in files {
        let parsed = AstParser::parse(file)?;

        let visitor = GraphVisitor::new(&mut graph, &parsed.module);
        visitor.visit_module(&parsed);
    }

    let renderer = PlantUmlRenderer;
    println!("{}", renderer.render(&graph));

    Ok(())
}
