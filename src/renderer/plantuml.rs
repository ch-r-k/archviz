use crate::model::{Edge, Node, NodeKind, Relation};
use crate::renderer::{DrawEdge, DrawNode, Renderer};

pub struct PlantUmlRenderer;

impl Renderer for PlantUmlRenderer {
    fn preamble(&self) -> String {
        "@startuml\n\n".to_string()
    }

    fn postamble(&self) -> String {
        "\n@enduml\n".to_string()
    }

    fn open_package(&self, name: &str) -> String {
        format!("package \"{}\" {{\n", name)
    }

    fn close_package(&self) -> String {
        "}\n\n".to_string()
    }
}

impl DrawNode for PlantUmlRenderer {
    fn draw_node(&self, node: &Node) -> String {
        let display = quoted(&node.display_name);
        // When the id differs from the display name, emit an
        // `as "<id>"` alias so cross-module edges — which reference the
        // fully-qualified id — resolve unambiguously in PlantUML.
        let alias = if node.id.as_str() != node.display_name {
            format!(" as {}", quoted(node.id.as_str()))
        } else {
            String::new()
        };

        match &node.kind {
            NodeKind::Struct => format!("class {}{}\n", display, alias),
            NodeKind::Trait => format!("interface {}{}\n", display, alias),
            NodeKind::Enum => format!("enum {}{}\n", display, alias),
            NodeKind::Impl {
                trait_name: Some(t),
            } => format!("class {}{} < {} >\n", display, alias, t),
            NodeKind::Impl { trait_name: None } => {
                format!("class {}{} \n", display, alias)
            }
            NodeKind::TypeAlias => format!("class {}{} < type >\n", display, alias),
            NodeKind::Synthetic { params: Some(params) } => {
                format!("class {}{} <{}>\n", display, alias, params)
            }
            NodeKind::Synthetic { params: None } => {
                format!("class {}{}\n", display, alias)
            }
        }
    }
}

impl DrawEdge for PlantUmlRenderer {
    fn draw_edge(&self, edge: &Edge) -> String {
        match edge.relation {
            Relation::Composition => format!(
                "{} --> {} : contains\n",
                quoted(edge.from.as_str()),
                quoted(edge.to.as_str())
            ),
            Relation::Implements => format!(
                "{} ..|> {}\n",
                quoted(edge.from.as_str()),
                quoted(edge.to.as_str())
            ),
            Relation::Specializes => format!(
                "{} <|-- {}\n",
                quoted(edge.from.as_str()),
                quoted(edge.to.as_str())
            ),
        }
    }
}

/// Wraps a name in PlantUML double-quotes when it contains characters that are
/// not valid in a bare identifier (e.g. `<`, `>`, `[`, `(`, `:`, spaces).
fn quoted(name: &str) -> String {
    if name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        name.to_string()
    } else {
        format!("\"{}\"", name)
    }
}
