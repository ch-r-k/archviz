use crate::model::*;

pub fn generate_plantuml(graph: &Graph) -> String {
    let mut out = String::new();

    out.push_str("@startuml\n\n");

    for node in &graph.nodes {
        match node.kind {
            NodeKind::Struct => {
                out.push_str(&format!("class {}\n", node.name));
            }
            NodeKind::Trait => {
                out.push_str(&format!("interface {}\n", node.name));
            }
        }
    }

    out.push('\n');

    for edge in &graph.edges {
        match edge.relation {
            Relation::Composition => {
                out.push_str(&format!("{} ..* {}\n", edge.from, edge.to))
            },
            Relation::Implements => {
                out.push_str(&format!("{} ..|> {}\n", edge.from, edge.to))
            }
        }
    }

    out.push_str("\n@enduml\n");
    out
}