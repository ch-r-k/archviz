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
                if let Some(params) = generic_params(expr) {
                    out.push_str(&format!(
                        "class {} <{}>\n",
                        quoted(&node.name),
                        params
                    ));
                } else {
                    out.push_str(&format!("class {}\n", quoted(&node.name)));
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

/// Renders the generic-parameter list of a synthetic type expression for
/// PlantUML's `class "Foo<Bar>" <Bar>` syntax. Returns `None` for expressions
/// that don't have a natural type-parameter list (simple names, traits, ...).
fn generic_params(expr: &TypeExpr) -> Option<String> {
    match expr {
        TypeExpr::Generic { args, .. } if !args.is_empty() => {
            let mut parts: Vec<String> = Vec::new();
            for a in args {
                parts.push(a.resolve_refs().type_name());
            }
            Some(parts.join(", "))
        }
        TypeExpr::Slice(inner) | TypeExpr::Array(inner) => {
            Some(inner.resolve_refs().type_name())
        }
        TypeExpr::Tuple(items) => {
            let mut parts: Vec<String> = Vec::new();
            for i in items {
                parts.push(i.resolve_refs().type_name());
            }
            Some(parts.join(", "))
        }
        _ => None,
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
