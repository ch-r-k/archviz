pub mod plantuml;
mod tests;

use crate::model::{Edge, Graph, Node};

pub trait DrawNode {
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

    fn render(&self, graph: &Graph) -> String {
        let mut out = self.preamble();
        for node in &graph.nodes {
            out.push_str(&self.draw_node(node));
        }
        out.push('\n');
        for edge in &graph.edges {
            out.push_str(&self.draw_edge(edge));
        }
        out.push_str(&self.postamble());
        out
    }
}
