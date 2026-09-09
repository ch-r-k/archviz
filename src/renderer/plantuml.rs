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
        // PlantUML rule (verified): in `class X as Y`, whichever side
        // is QUOTED is the display; a bare identifier is the alias. If
        // both sides are bare identifiers PlantUML errors out
        // ("Some diagram description contains errors"). So whenever we
        // emit an alias we always quote the display, even if the display
        // itself would be a valid bare identifier.
        //
        // Aliases must be bare (a quoted string on the right of `as` is
        // treated as another display), so we sanitize the FQ id
        // (`model::Node` → `model__Node`).
        let (display, alias) = if node.id.as_str() != node.display_name {
            (
                format!("\"{}\"", node.display_name),
                format!(" as {}", sanitize_alias(node.id.as_str())),
            )
        } else {
            (quoted_display(&node.display_name), String::new())
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
            NodeKind::Package => {
                // A collapsed module: render as an empty package block.
                // `display` is already quoted; `alias` is either
                // ` as <sanitized-id>` or empty when the id matches the
                // display.
                format!("package {}{} {{\n}}\n\n", display, alias)
            }
        }
    }
}

impl DrawEdge for PlantUmlRenderer {
    fn draw_edge(&self, edge: &Edge) -> String {
        let from = edge_ref(edge.from.as_str());
        let to = edge_ref(edge.to.as_str());
        match edge.relation {
            Relation::Composition => format!("{} --> {} : contains\n", from, to),
            Relation::Implements => format!("{} ..|> {}\n", from, to),
            Relation::Specializes => format!("{} ..|> {}\n", from, to),
        }
    }
}

/// How an edge references a node. Fully-qualified ids (`model::Node`)
/// resolve via the sanitized bare alias (`model_Node`) emitted by
/// `draw_node`. Bare ids that happen to contain non-identifier
/// characters (`Vec<String>`, `Option<T>`) have no alias — the display
/// itself is the reference, so we quote it.
fn edge_ref(id: &str) -> String {
    if id.contains("::") {
        sanitize_alias(id)
    } else {
        quoted_display(id)
    }
}

/// Wraps a display name in PlantUML double-quotes when it contains
/// characters that are not valid in a bare identifier (e.g. `<`, `>`,
/// `[`, `(`, `:`, spaces).
fn quoted_display(name: &str) -> String {
    if name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        name.to_string()
    } else {
        format!("\"{}\"", name)
    }
}

/// Rewrites a fully-qualified id into a bare PlantUML identifier
/// suitable as an alias after `as`. PlantUML aliases can't be quoted
/// (a quoted string on the right of `as` is treated as a display), so
/// every non-identifier character becomes `_`.
fn sanitize_alias(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}
