use crate::model::Graph;
use crate::resolution::ResolutionContext;

/// A post-parse pass that mutates the [`Graph`] in place.
///
/// Multiple enrichers run in the order they were registered on the
/// [`crate::pipeline::PipelineBuilder`]. Each one should be independently
/// testable and idempotent. The shared [`ResolutionContext`] carries
/// run-level inputs (the cargo crate index); passes that don't need them
/// can ignore the argument.
pub trait GraphEnricher {
    fn enrich(&self, graph: &mut Graph, ctx: &ResolutionContext<'_>);
}
