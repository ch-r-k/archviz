mod model;
mod extractor;
mod renderer;

use anyhow::Result;
use std::ffi::OsStr;
use syn::visit::Visit;
use walkdir::WalkDir;

use crate::extractor::extractor::Extractor;
use crate::renderer::plant_uml::generate_plantuml;

fn main() -> Result<()> {
    let root = std::env::args()
        .nth(1)
        .expect("Usage: archviz <path>");

    let mut extractor = Extractor::new();

    for entry in WalkDir::new(root) {
        let entry = entry?;

        if entry.file_type().is_file()
            && entry.path().extension() == Some(OsStr::new("rs"))
        {
            let source = std::fs::read_to_string(entry.path())?;
            let ast = syn::parse_file(&source)?;
            extractor.visit_file(&ast);
        }
    }

    let uml = generate_plantuml(&extractor.graph);

    println!("{uml}");

    Ok(())
}