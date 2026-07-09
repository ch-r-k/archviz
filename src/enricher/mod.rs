pub mod origin;
pub mod origin_resolver;
pub mod type_expander;

use crate::model::Graph;

pub trait GraphEnricher {
    fn enrich(&self, graph: &mut Graph);
}
