pub mod origin_resolver;

use crate::model::Graph;

pub trait GraphEnricher {
    fn enrich(&self, graph: &mut Graph);
}
