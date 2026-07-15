use std::collections::HashSet;

use super::node::NodeId;
use super::{Edge, Node};

/// Owns all nodes and edges produced by the pipeline stages.
///
/// A `HashSet<NodeId>` index over node ids is kept alongside `nodes`
/// so existence checks during construction stay O(1).
#[derive(Default)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    node_index: HashSet<NodeId>,
}

impl Graph {
    /// Adds `node` unconditionally (used for source-level definitions
    /// where the caller expects every parsed item to be represented,
    /// even if a same-named node already exists).
    pub fn add_node(&mut self, node: Node) {
        self.node_index.insert(node.id.clone());
        self.nodes.push(node);
    }

    /// Adds `node` to the graph if a node with the same id is not
    /// already present. Returns `true` when the node was inserted.
    /// Intended for synthetic nodes that must be deduplicated.
    pub fn push_node(&mut self, node: Node) -> bool {
        if !self.node_index.insert(node.id.clone()) {
            return false;
        }
        self.nodes.push(node);
        true
    }

    pub fn has_node(&self, id: &NodeId) -> bool {
        self.node_index.contains(id)
    }
}
