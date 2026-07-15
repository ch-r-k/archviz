#![cfg(test)]

use crate::enricher::GraphEnricher;
use crate::enricher::origin_resolver::OriginResolver;
use crate::model::{Edge, Graph, Node, NodeKind, Relation};

fn node(name: &str, kind: NodeKind, module_path: Vec<String>) -> Node {
    Node {
        name: name.into(),
        kind,
        module_path,
    }
}

#[test]
fn classifies_std_type_under_std_package() {
    let mut graph = Graph::default();
    graph.add_node(node("User", NodeKind::Struct, vec!["m".into()]));
    graph.edges.push(Edge {
        from: "User".into(),
        to: "String".into(),
        relation: Relation::Composition,
    });

    OriginResolver.enrich(&mut graph);

    let string_node = graph
        .nodes
        .iter()
        .find(|n| n.name == "String")
        .expect("String node emitted");
    assert_eq!(string_node.module_path, vec!["std".to_string()]);
}

#[test]
fn classifies_unknown_as_external() {
    let mut graph = Graph::default();
    graph.edges.push(Edge {
        from: "A".into(),
        to: "MyCrateType".into(),
        relation: Relation::Composition,
    });

    OriginResolver.enrich(&mut graph);

    let n = graph
        .nodes
        .iter()
        .find(|n| n.name == "MyCrateType")
        .expect("external node emitted");
    assert_eq!(n.module_path, vec!["external".to_string()]);
}

#[test]
fn does_not_emit_local_stub() {
    let mut graph = Graph::default();
    graph.add_node(node("User", NodeKind::Struct, vec!["m".into()]));
    graph.add_node(node("Profile", NodeKind::Struct, vec!["m".into()]));
    graph.edges.push(Edge {
        from: "User".into(),
        to: "Profile".into(),
        relation: Relation::Composition,
    });

    let before = graph.nodes.len();
    OriginResolver.enrich(&mut graph);

    // Should not have duplicated the local Profile node.
    assert_eq!(graph.nodes.len(), before);
}

#[test]
fn skips_type_parameter_names() {
    let mut graph = Graph::default();
    graph.edges.push(Edge {
        from: "Container".into(),
        to: "T".into(),
        relation: Relation::Composition,
    });
    graph.edges.push(Edge {
        from: "Container".into(),
        to: "K1".into(),
        relation: Relation::Composition,
    });

    OriginResolver.enrich(&mut graph);

    assert!(graph.nodes.iter().all(|n| n.name != "T" && n.name != "K1"));
}
