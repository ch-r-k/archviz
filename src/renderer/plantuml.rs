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
        match &node.kind {
            NodeKind::Struct => format!("class {}\n", quoted(&node.name)),
            NodeKind::Trait => format!("interface {}\n", quoted(&node.name)),
            NodeKind::Enum => format!("enum {}\n", quoted(&node.name)),
            NodeKind::Impl {
                trait_name: Some(t),
            } => format!("class {} < {} >\n", quoted(&node.name), t),
            NodeKind::Impl { trait_name: None } => {
                format!("class {} \n", quoted(&node.name))
            }
            NodeKind::TypeAlias => format!("class {} < type >\n", quoted(&node.name)),
            NodeKind::Synthetic { params: Some(params) } => {
                format!("class {} <{}>\n", quoted(&node.name), params)
            }
            NodeKind::Synthetic { params: None } => {
                format!("class {}\n", quoted(&node.name))
            }
        }
    }
}

impl DrawEdge for PlantUmlRenderer {
    fn draw_edge(&self, edge: &Edge) -> String {
        match edge.relation {
            Relation::Composition => format!(
                "{} --> {} : contains\n",
                quoted(&edge.from),
                quoted(&edge.to)
            ),
            Relation::Implements => format!("{} ..|> {}\n", quoted(&edge.from), quoted(&edge.to)),
            Relation::Specializes => format!("{} <|-- {}\n", quoted(&edge.from), quoted(&edge.to)),
        }
    }
}

/// Wraps a name in PlantUML double-quotes when it contains characters that are
/// not valid in a bare identifier (e.g. `<`, `>`, `[`, `(`, spaces).
fn quoted(name: &str) -> String {
    if name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        name.to_string()
    } else {
        format!("\"{}\"", name)
    }
}
