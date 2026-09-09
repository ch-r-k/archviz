#![cfg(test)]

use crate::enricher::GraphEnricher;
use crate::enricher::edge_target_resolver::EdgeTargetResolver;
use crate::enricher::origin_resolver::OriginResolver;
use crate::model::node::NodeId;
use crate::model::{Edge, Graph, Node, NodeKind, Relation};

fn node(name: &str, kind: NodeKind, module_path: Vec<String>) -> Node {
    Node {
        id: NodeId::from_parts(&module_path, name),
        display_name: name.into(),
        kind,
        module_path,
    }
}

fn fq_node(id: &str, display: &str, module: &[&str]) -> Node {
    Node {
        id: NodeId(id.into()),
        display_name: display.into(),
        kind: NodeKind::Struct,
        module_path: module.iter().map(|s| s.to_string()).collect(),
    }
}

#[test]
fn classifies_std_type_under_std_package() {
    let mut graph = Graph::default();
    graph.add_node(node("User", NodeKind::Struct, vec!["m".into()]));
    graph.edges.push(Edge {
        from: NodeId("m::User".into()),
        to: NodeId::bare("String"),
        relation: Relation::Composition,
    });

    OriginResolver.enrich(&mut graph);

    let string_node = graph
        .nodes
        .iter()
        .find(|n| n.display_name == "String")
        .expect("String node emitted");
    assert_eq!(string_node.module_path, vec!["std".to_string()]);
}

#[test]
fn classifies_unknown_as_external() {
    let mut graph = Graph::default();
    graph.edges.push(Edge {
        from: NodeId::bare("A"),
        to: NodeId::bare("MyCrateType"),
        relation: Relation::Composition,
    });

    OriginResolver.enrich(&mut graph);

    let n = graph
        .nodes
        .iter()
        .find(|n| n.display_name == "MyCrateType")
        .expect("external node emitted");
    assert_eq!(n.module_path, vec!["external".to_string()]);
}

#[test]
fn does_not_emit_local_stub() {
    let mut graph = Graph::default();
    graph.add_node(node("User", NodeKind::Struct, vec!["m".into()]));
    graph.add_node(node("Profile", NodeKind::Struct, vec!["m".into()]));
    graph.edges.push(Edge {
        from: NodeId("m::User".into()),
        to: NodeId("m::Profile".into()),
        relation: Relation::Composition,
    });

    let before = graph.nodes.len();
    OriginResolver.enrich(&mut graph);
    assert_eq!(graph.nodes.len(), before);
}

#[test]
fn skips_type_parameter_names() {
    let mut graph = Graph::default();
    graph.edges.push(Edge {
        from: NodeId::bare("Container"),
        to: NodeId::bare("T"),
        relation: Relation::Composition,
    });
    graph.edges.push(Edge {
        from: NodeId::bare("Container"),
        to: NodeId::bare("K1"),
        relation: Relation::Composition,
    });

    OriginResolver.enrich(&mut graph);

    assert!(
        graph
            .nodes
            .iter()
            .all(|n| n.display_name != "T" && n.display_name != "K1")
    );
}

#[test]
fn edge_target_resolver_resolves_unique_display_name() {
    let mut g = Graph::default();
    g.add_node(fq_node("m::Foo", "Foo", &["m"]));
    g.add_node(fq_node("m::Bar", "Bar", &["m"]));
    g.edges.push(Edge {
        from: NodeId("m::Bar".into()),
        to: NodeId::bare("Foo"),
        relation: Relation::Composition,
    });

    EdgeTargetResolver.enrich(&mut g);
    assert_eq!(g.edges[0].to.as_str(), "m::Foo");
}

#[test]
fn edge_target_resolver_prefers_same_module() {
    let mut g = Graph::default();
    g.add_node(fq_node("a::Foo", "Foo", &["a"]));
    g.add_node(fq_node("b::Foo", "Foo", &["b"]));
    g.add_node(fq_node("b::Bar", "Bar", &["b"]));
    g.edges.push(Edge {
        from: NodeId("b::Bar".into()),
        to: NodeId::bare("Foo"),
        relation: Relation::Composition,
    });

    EdgeTargetResolver.enrich(&mut g);
    assert_eq!(g.edges[0].to.as_str(), "b::Foo");
}

#[test]
fn edge_target_resolver_leaves_unresolved_bare() {
    let mut g = Graph::default();
    g.add_node(fq_node("m::Bar", "Bar", &["m"]));
    g.edges.push(Edge {
        from: NodeId("m::Bar".into()),
        to: NodeId::bare("String"),
        relation: Relation::Composition,
    });

    EdgeTargetResolver.enrich(&mut g);
    assert_eq!(g.edges[0].to.as_str(), "String");
}

