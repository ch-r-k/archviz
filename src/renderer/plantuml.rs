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
                out.push_str(&format!("class {} \n", quoted(&node.name)));
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
        let target = type_name(resolve_refs(&edge.to));
        match edge.relation {
            Relation::Composition => format!("{} *-- {}\n", quoted(&edge.from), quoted(&target)),
            Relation::Implements => format!("{} ..|> {}\n", quoted(&edge.from), quoted(&target)),
        }
    }
}

/// Strips top-level `&` references to reach the underlying type.
fn resolve_refs(expr: &TypeExpr) -> &TypeExpr {
    match expr {
        TypeExpr::Reference(inner) => resolve_refs(inner),
        other => other,
    }
}

/// Returns the display name for a type expression.
fn type_name(expr: &TypeExpr) -> String {
    match expr {
        TypeExpr::Simple(name) => name.clone(),
        TypeExpr::Generic { base, args } => {
            if args.is_empty() {
                base.clone()
            } else {
                let rendered = args
                    .iter()
                    .map(|a| type_name(resolve_refs(a)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}<{}>", base, rendered)
            }
        }
        TypeExpr::Reference(inner) => type_name(inner),
        TypeExpr::Slice(inner) => format!("[{}]", type_name(resolve_refs(inner))),
        TypeExpr::Array(inner) => format!("[{}; N]", type_name(resolve_refs(inner))),
        TypeExpr::Tuple(items) => {
            let rendered = items
                .iter()
                .map(|i| type_name(resolve_refs(i)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("({})", rendered)
        }
        TypeExpr::DynTrait(traits) => format!("dyn {}", traits.join(" + ")),
        TypeExpr::ImplTrait(traits) => format!("impl {}", traits.join(" + ")),
        TypeExpr::Unknown => "_".to_string(),
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
