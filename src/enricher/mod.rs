pub mod origin_resolver;
mod tests;

use crate::model::Graph;

pub trait GraphEnricher {
    fn enrich(&self, graph: &mut Graph);
}
