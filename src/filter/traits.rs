//! Trait for the pipeline's dedicated *filter* stage.
//!
//! A `Filter` runs after all [`GraphEnricher`](crate::enricher::GraphEnricher)s
//! and before the [`Renderer`](crate::renderer::Renderer). Unlike
//! enrichers — which *add* information (resolved edge targets,
//! origins, …) — filters *reshape or reduce* the graph based on user
//! intent (include/exclude/collapse).

use crate::model::Graph;

pub trait Filter {
    fn apply(&self, graph: &mut Graph);
}
