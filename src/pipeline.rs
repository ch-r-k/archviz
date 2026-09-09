use anyhow::{anyhow, Result};

use crate::enricher::GraphEnricher;
use crate::enricher::edge_target_resolver::EdgeTargetResolver;
use crate::enricher::origin_resolver::OriginResolver;
use crate::error::ArchError;
use crate::filter::{Filter, FilterOptions};
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
    filters: Vec<Box<dyn Filter>>,
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

        for filter in &self.filters {
            filter.apply(&mut graph);
        }

        if graph.nodes.is_empty() {
            eprintln!(
                "archviz: no nodes remain after filtering; \
                 emitting an empty diagram"
            );
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
    filters: Vec<Box<dyn Filter>>,
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
            filters: Vec::new(),
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

    /// Appends a filter stage. Filters run after all enrichers and
    /// before the renderer.
    #[allow(dead_code)]
    pub fn with_filter(mut self, filter: impl Filter + 'static) -> Self {
        self.filters.push(Box::new(filter));
        self
    }

    /// Front-end entry point: accept plain-data options (raw pattern
    /// strings + optional depth) and build a filter stage internally.
    ///
    /// This is the *only* API the CLI needs to touch — CLI code stays
    /// unaware of [`crate::filter::ModulePattern`] / [`crate::filter::FilterSpec`].
    /// A no-op if `options` is empty.
    pub fn with_filter_options(mut self, options: FilterOptions) -> Result<Self, ArchError> {
        if options.is_empty() {
            return Ok(self);
        }
        let filter = options.build()?;
        self.filters.push(Box::new(filter));
        Ok(self)
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
            filters: self.filters,
            renderer: self.renderer,
        }
    }
}
