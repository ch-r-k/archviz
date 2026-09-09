use crate::model::{Edge, Graph, Node};
use crate::renderer::module_tree::ModuleTree;

/// Renders a single [`Node`] in isolation. Module-path grouping is
/// handled by [`Renderer::render`] via [`Renderer::open_package`] /
/// [`Renderer::close_package`], not here.
pub trait DrawNode {
    fn draw_node(&self, node: &Node) -> String;
}

/// Renders a single [`Edge`].
pub trait DrawEdge {
    fn draw_edge(&self, edge: &Edge) -> String;
}

/// Top-level rendering trait. Combines [`DrawNode`] + [`DrawEdge`] and
/// adds hooks for the document preamble/postamble and package
/// open/close markers.
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
