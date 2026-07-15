pub mod plantuml;
mod tests;

use std::collections::BTreeMap;

use crate::model::{Edge, Graph, Node};

pub trait DrawNode {
    /// Draws just the node itself, without any surrounding package
    /// blocks — module grouping is handled by [`Renderer::render`].
    fn draw_node(&self, node: &Node) -> String;
}

pub trait DrawEdge {
    fn draw_edge(&self, edge: &Edge) -> String;
}

pub trait Renderer: DrawNode + DrawEdge {
    fn preamble(&self) -> String {
        String::new()
    }

    fn postamble(&self) -> String {
        String::new()
    }

    fn open_package(&self, _name: &str) -> String {
        String::new()
    }

    fn close_package(&self) -> String {
        String::new()
    }

    fn render(&self, graph: &Graph) -> String {
        let mut out = self.preamble();

        let tree = ModuleTree::build(&graph.nodes);
        tree.render_into(self, &mut out);

        out.push('\n');
        for edge in &graph.edges {
            out.push_str(&self.draw_edge(edge));
        }
        out.push_str(&self.postamble());
        out
    }
}

/// A tree of module segments where each node keeps the source nodes
/// declared at that exact module path. Used by [`Renderer::render`] to
/// emit one `package` block per unique module segment (rather than one
/// per graph node).
#[derive(Default)]
struct ModuleTree<'a> {
    children: BTreeMap<String, ModuleTree<'a>>,
    nodes: Vec<&'a Node>,
}

impl<'a> ModuleTree<'a> {
    fn build(nodes: &'a [Node]) -> Self {
        let mut root = ModuleTree::default();
        for n in nodes {
            root.insert(&n.module_path, n);
        }
        root
    }

    fn insert(&mut self, segments: &[String], node: &'a Node) {
        match segments.split_first() {
            None => self.nodes.push(node),
            Some((head, tail)) => {
                self.children.entry(head.clone()).or_default().insert(tail, node);
            }
        }
    }

    fn render_into<R: Renderer + ?Sized>(&self, renderer: &R, out: &mut String) {
        for node in &self.nodes {
            out.push_str(&renderer.draw_node(node));
        }
        for (name, child) in &self.children {
            out.push_str(&renderer.open_package(name));
            child.render_into(renderer, out);
            out.push_str(&renderer.close_package());
        }
    }
}
