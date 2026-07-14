use anyhow::Result;

use crate::enricher::GraphEnricher;
use crate::enricher::origin_resolver::OriginResolver;
use crate::model::Graph;
use crate::parser::{AstParser, Parser};
use crate::parser::visitor::GraphVisitor;
use crate::project::ProjectLoader;
use crate::renderer::{Renderer};
use crate::renderer::plantuml::PlantUmlRenderer;

pub struct Pipeline {
    root: String,
    parser: Box<dyn Parser>,
    enrichers: Vec<Box<dyn GraphEnricher>>,
    renderer: Box<dyn Renderer>,
}

impl Pipeline {
    pub fn builder(root: impl Into<String>) -> PipelineBuilder {
        PipelineBuilder::new(root)
    }

    pub fn run(&self) -> Result<String> {
        let loader = ProjectLoader::new(self.root.clone());
        let files = loader.load()?;

        let mut graph = Graph::default();

        for file in files {
            let parsed = self.parser.parse(file)?;

            let visitor = GraphVisitor::new(&mut graph, &parsed.module_path);
            visitor.visit_module(&parsed);
        }

        for enricher in &self.enrichers {
            enricher.enrich(&mut graph);
        }

        Ok(self.renderer.render(&graph))
    }
}

pub struct PipelineBuilder {
    root: String,
    parser: Box<dyn Parser>,
    enrichers: Vec<Box<dyn GraphEnricher>>,
    renderer: Box<dyn Renderer>,
}

impl PipelineBuilder {
    pub fn new(root: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            parser: Box::new(AstParser),
            enrichers: vec![Box::new(OriginResolver)],
            renderer: Box::new(PlantUmlRenderer),
        }
    }

    pub fn with_parser(mut self, parser: impl Parser + 'static) -> Self {
        self.parser = Box::new(parser);
        self
    }

    pub fn with_enricher(mut self, enricher: impl GraphEnricher + 'static) -> Self {
        self.enrichers.push(Box::new(enricher));
        self
    }

    pub fn with_renderer(mut self, renderer: impl Renderer + 'static) -> Self {
        self.renderer = Box::new(renderer);
        self
    }

    pub fn build(self) -> Pipeline {
        Pipeline {
            root: self.root,
            parser: self.parser,
            enrichers: self.enrichers,
            renderer: self.renderer,
        }
    }
}
