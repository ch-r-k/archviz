use crate::model::{Edge, Node, NodeKind, Relation, TypeExpr};
use crate::renderer::{DrawEdge, DrawNode, Renderer};

pub struct PlantUmlRenderer;

impl Renderer for PlantUmlRenderer {
    fn preamble(&self) -> String {
        "@startuml\n\n".to_string()
    }

    fn postamble(&self) -> String {
        "\n@enduml\n".to_string()
    }
}

impl DrawNode for PlantUmlRenderer {
    fn draw_node(&self, node: &Node) -> String {
        let mut out = String::new();

        for module in &node.module_path {
            out.push_str(&format!("package \"{}\" {{\n", module));
        }

        match &node.kind {
            NodeKind::Struct => {
                out.push_str(&format!("class {}\n", quoted(&node.name)));
            }
            NodeKind::Trait => {
                out.push_str(&format!("interface {}\n", quoted(&node.name)));
            }
            NodeKind::Enum => {
                out.push_str(&format!("enum {}\n", quoted(&node.name)));
            }
            NodeKind::Impl {
                trait_name: Some(t),
            } => {
                out.push_str(&format!("class {} < {} >\n", quoted(&node.name), t));
            }
            NodeKind::Impl { trait_name: None } => {
                out.push_str(&format!("class {} \n", quoted(&node.name)));
            }
            NodeKind::TypeAlias => {
                out.push_str(&format!("class {} < type >\n", quoted(&node.name)));
            }
            NodeKind::Synthetic { expr: Some(expr) } => {
                out.push_str(&format!("class {}\n", quoted(&node.name)));
                
                // Add a note explaining the generic type parameters
                if let Some(note) = generic_type_note(expr) {
                    out.push_str(&format!(
                        "note right of {} : {}\n",
                        quoted(&node.name),
                        note
                    ));
                }
            }
            NodeKind::Synthetic { expr: None } => {
                out.push_str(&format!("class {} \n", quoted(&node.name)));
            }
        }

        for _ in &node.module_path {
            out.push_str("}\n\n");
        }

        out
    }
}

impl DrawEdge for PlantUmlRenderer {
    fn draw_edge(&self, edge: &Edge) -> String {
        let target = edge.to.resolve_refs().type_name();
        match edge.relation {
            Relation::Composition => format!(
                "{} --> {} : contains\n",
                quoted(&edge.from),
                quoted(&target)
            ),
            Relation::Implements => format!("{} ..|> {}\n", quoted(&edge.from), quoted(&target)),
            Relation::Specializes => format!("{} <|-- {}\n", quoted(&edge.from), quoted(&target)),
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

/// Generates a PlantUML note for a synthetic generic type, showing the base type
/// and type parameters. For example, `Vec<String>` → "Generic Vec<T> with T = String"
fn generic_type_note(expr: &TypeExpr) -> Option<String> {
    match expr {
        TypeExpr::Generic { base, args } if !args.is_empty() => {
            let args_str = args
                .iter()
                .map(|a| a.resolve_refs().type_name())
                .collect::<Vec<_>>()
                .join(", ");
            Some(format!("Generic {}<<T>> with T = {}", base, args_str))
        }
        TypeExpr::Slice(inner) => Some(format!("Slice of {}", inner.resolve_refs().type_name())),
        TypeExpr::Array(inner) => Some(format!("Array of {}", inner.resolve_refs().type_name())),
        TypeExpr::Tuple(items) => {
            let items_str = items
                .iter()
                .map(|i| i.resolve_refs().type_name())
                .collect::<Vec<_>>()
                .join(", ");
            Some(format!("Tuple of ({})", items_str))
        }
        _ => None,
    }
}
