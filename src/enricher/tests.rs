#![cfg(test)]

use crate::enricher::GraphEnricher;
use crate::enricher::edge_target_resolver::EdgeTargetResolver;
use crate::enricher::origin_resolver::OriginResolver;
use crate::model::node::NodeId;
use crate::model::{Edge, Graph, Node, NodeKind, Relation};
use crate::resolution::{CrateIndex, ResolutionContext};

fn empty_ctx() -> ResolutionContext<'static> {
    ResolutionContext { crate_index: None }
}

fn index_with(crates: &[&str]) -> CrateIndex {
    let names = crates.iter().map(|c| c.to_string()).collect();
    CrateIndex::new(names)
}

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

    OriginResolver.enrich(&mut graph, &empty_ctx());

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

    OriginResolver.enrich(&mut graph, &empty_ctx());

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
    OriginResolver.enrich(&mut graph, &empty_ctx());
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

    OriginResolver.enrich(&mut graph, &empty_ctx());

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

    EdgeTargetResolver.enrich(&mut g, &empty_ctx());
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

    EdgeTargetResolver.enrich(&mut g, &empty_ctx());
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

    EdgeTargetResolver.enrich(&mut g, &empty_ctx());
    assert_eq!(g.edges[0].to.as_str(), "String");
}

#[test]
fn origin_resolver_nests_external_crate_path() {
    let mut g = Graph::default();
    g.add_node(fq_node("m::A", "A", &["m"]));
    g.edges.push(Edge {
        from: NodeId("m::A".into()),
        to: NodeId::bare("anyhow::Error"),
        relation: Relation::Composition,
    });

    let index = index_with(&["anyhow"]);
    let ctx = ResolutionContext {
        crate_index: Some(&index),
    };
    OriginResolver.enrich(&mut g, &ctx);

    let n = g
        .nodes
        .iter()
        .find(|n| n.display_name == "Error")
        .expect("external stub emitted");
    assert_eq!(n.module_path, vec!["external".to_string(), "anyhow".to_string()]);
    assert_eq!(n.id.as_str(), "anyhow::Error");
}

#[test]
fn origin_resolver_classifies_lang_root_path_as_std() {
    let mut g = Graph::default();
    g.add_node(fq_node("m::A", "A", &["m"]));
    g.edges.push(Edge {
        from: NodeId("m::A".into()),
        to: NodeId::bare("std::collections::HashMap"),
        relation: Relation::Composition,
    });

    OriginResolver.enrich(&mut g, &empty_ctx());

    let n = g
        .nodes
        .iter()
        .find(|n| n.display_name == "HashMap")
        .expect("std stub emitted");
    assert_eq!(n.module_path, vec!["std".to_string()]);
}

#[test]
fn origin_resolver_skips_crate_qualified_path() {
    let mut g = Graph::default();
    g.add_node(fq_node("m::A", "A", &["m"]));
    g.edges.push(Edge {
        from: NodeId("m::A".into()),
        to: NodeId::bare("crate::Foo"),
        relation: Relation::Composition,
    });

    let before = g.nodes.len();
    OriginResolver.enrich(&mut g, &empty_ctx());
    assert_eq!(g.nodes.len(), before, "no stub for crate:: paths");
}

#[test]
fn origin_resolver_flat_external_without_index() {
    let mut g = Graph::default();
    g.edges.push(Edge {
        from: NodeId::bare("A"),
        to: NodeId::bare("serde::Serialize"),
        relation: Relation::Composition,
    });

    OriginResolver.enrich(&mut g, &empty_ctx());

    let n = g
        .nodes
        .iter()
        .find(|n| n.display_name == "Serialize")
        .expect("flat external stub emitted");
    assert_eq!(n.module_path, vec!["external".to_string()]);
}

#[test]
fn resolution_ctx_is_unused_by_edge_target_resolver() {
    let mut g = Graph::default();
    g.add_node(fq_node("m::Foo", "Foo", &["m"]));
    g.edges.push(Edge {
        from: NodeId::bare("A"),
        to: NodeId::bare("Foo"),
        relation: Relation::Composition,
    });

    let index = index_with(&["anyhow"]);
    let ctx = ResolutionContext {
        crate_index: Some(&index),
    };
    EdgeTargetResolver.enrich(&mut g, &ctx);
    assert_eq!(g.edges[0].to.as_str(), "m::Foo");
}

