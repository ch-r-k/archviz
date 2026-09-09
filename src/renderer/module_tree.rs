//! A tree of module segments used by [`crate::renderer::Renderer::render`]
//! to group graph nodes into one `package` block per unique module
//! segment (rather than one wrapper per node).

use std::collections::BTreeMap;

use crate::model::Node;
use crate::renderer::traits::Renderer;

#[derive(Default)]
pub(crate) struct ModuleTree<'a> {
    children: BTreeMap<String, ModuleTree<'a>>,
    nodes: Vec<&'a Node>,
}

impl<'a> ModuleTree<'a> {
    pub(crate) fn build(nodes: &'a [Node]) -> Self {
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
                self.children
                    .entry(head.clone())
                    .or_default()
                    .insert(tail, node);
            }
        }
    }

    pub(crate) fn render_into<R: Renderer + ?Sized>(&self, renderer: &R, out: &mut String) {
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
