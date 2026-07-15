use anyhow::{anyhow, Result};

use crate::enricher::GraphEnricher;
use crate::enricher::edge_target_resolver::EdgeTargetResolver;
use crate::enricher::origin_resolver::OriginResolver;
use crate::model::Graph;
use crate::parser::visitor::GraphVisitor;
use crate::parser::{AstParser, Parser};
use crate::project::{ProjectLoader, SourceFile};
use crate::renderer::Renderer;
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
        let total = files.len();

        let mut graph = Graph::default();
        let mut succeeded = 0usize;

        for file in files {
            match self.parse_and_visit(file, &mut graph) {
                Ok(()) => succeeded += 1,
                Err((path, err)) => {
                    eprintln!("archviz: skipping {}: {}", path.display(), err);
                }
            }
        }

        if total > 0 && succeeded == 0 {
            return Err(anyhow!("no source files could be parsed"));
        }

        for enricher in &self.enrichers {
            enricher.enrich(&mut graph);
        }

        Ok(self.renderer.render(&graph))
    }

    fn parse_and_visit(
        &self,
        file: SourceFile,
        graph: &mut Graph,
    ) -> Result<(), (std::path::PathBuf, crate::error::ArchError)> {
        let path = file.path.clone();
        let parsed = self.parser.parse(file).map_err(|e| (path.clone(), e))?;

        let visitor = GraphVisitor::new(graph, &parsed.module_path);
        visitor.visit_module(&parsed);
        Ok(())
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
            // Order matters: EdgeTargetResolver must run before
            // OriginResolver so that std/external stubs are only
            // created for genuinely unresolved bare targets.
            enrichers: vec![Box::new(EdgeTargetResolver), Box::new(OriginResolver)],
            renderer: Box::new(PlantUmlRenderer),
        }
    }

    #[allow(dead_code)]
    pub fn with_parser(mut self, parser: impl Parser + 'static) -> Self {
        self.parser = Box::new(parser);
        self
    }

    #[allow(dead_code)]
    pub fn with_enricher(mut self, enricher: impl GraphEnricher + 'static) -> Self {
        self.enrichers.push(Box::new(enricher));
        self
    }

    #[allow(dead_code)]
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
