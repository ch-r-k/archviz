use crate::model::Graph;

/// A post-parse pass that mutates the [`Graph`] in place.
///
/// Multiple enrichers run in the order they were registered on the
/// [`crate::pipeline::PipelineBuilder`]. Each one should be independently
/// testable and idempotent.
pub trait GraphEnricher {
    fn enrich(&self, graph: &mut Graph);
}
