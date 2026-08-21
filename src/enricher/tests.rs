#![cfg(test)]

use crate::enricher::GraphEnricher;
use crate::enricher::edge_target_resolver::EdgeTargetResolver;
use crate::enricher::module_filter::ModuleFilter;
use crate::enricher::origin_resolver::OriginResolver;
use crate::filter::pattern::ModulePattern;
use crate::filter::spec::FilterSpec;
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

fn pat(s: &str) -> ModulePattern {
    ModulePattern::parse(s).expect("valid pattern")
}

fn mods(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

#[test]
fn module_filter_drops_excluded_nodes_and_dangling_edges() {
    let mut g = Graph::default();
    g.add_node(node("Public", NodeKind::Struct, mods(&["a", "b", "public"])));
    g.add_node(node(
        "Internal",
        NodeKind::Struct,
        mods(&["a", "b", "internal"]),
    ));
    g.edges.push(Edge {
        from: NodeId::from_parts(&mods(&["a", "b", "public"]), "Public"),
        to: NodeId::from_parts(&mods(&["a", "b", "internal"]), "Internal"),
        relation: Relation::Composition,
    });

    let filter = ModuleFilter::new(FilterSpec {
        exclude: vec![pat("**::internal")],
        ..FilterSpec::default()
    });
    filter.enrich(&mut g);

    assert_eq!(g.nodes.len(), 1);
    assert_eq!(g.nodes[0].display_name, "Public");
    assert!(g.edges.is_empty(), "edges: {:?}", g.edges);
}

#[test]
fn module_filter_include_only_keeps_matches() {
    let mut g = Graph::default();
    g.add_node(node("Foo", NodeKind::Struct, mods(&["api"])));
    g.add_node(node("Bar", NodeKind::Struct, mods(&["internal"])));

    let filter = ModuleFilter::new(FilterSpec {
        include: vec![pat("api::**")],
        ..FilterSpec::default()
    });
    filter.enrich(&mut g);

    let names: Vec<_> = g.nodes.iter().map(|n| n.display_name.clone()).collect();
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn module_filter_empty_spec_is_noop() {
    let mut g = Graph::default();
    g.add_node(node("A", NodeKind::Struct, mods(&["m"])));
    g.edges.push(Edge {
        from: NodeId::from_parts(&mods(&["m"]), "A"),
        to: NodeId::bare("String"),
        relation: Relation::Composition,
    });

    let before_nodes = g.nodes.len();
    let before_edges = g.edges.len();
    ModuleFilter::new(FilterSpec::default()).enrich(&mut g);
    assert_eq!(g.nodes.len(), before_nodes);
    assert_eq!(g.edges.len(), before_edges);
}

#[test]
fn module_filter_collapse_replaces_subtree_with_package() {
    let mut g = Graph::default();
    g.add_node(node("Foo", NodeKind::Struct, mods(&["a", "b"])));
    g.add_node(node("Bar", NodeKind::Struct, mods(&["a", "b"])));
    g.add_node(node("Baz", NodeKind::Struct, mods(&["c"])));

    let foo = NodeId::from_parts(&mods(&["a", "b"]), "Foo");
    let bar = NodeId::from_parts(&mods(&["a", "b"]), "Bar");
    let baz = NodeId::from_parts(&mods(&["c"]), "Baz");

    g.edges.push(Edge {
        from: foo.clone(),
        to: bar.clone(),
        relation: Relation::Composition,
    });
    g.edges.push(Edge {
        from: bar.clone(),
        to: baz.clone(),
        relation: Relation::Composition,
    });

    let filter = ModuleFilter::new(FilterSpec {
        collapse: vec![pat("a::b")],
        ..FilterSpec::default()
    });
    filter.enrich(&mut g);

    // Nodes: only the package `a::b` and the survivor `c::Baz`.
    let mut names: Vec<_> = g.nodes.iter().map(|n| n.display_name.clone()).collect();
    names.sort();
    assert_eq!(names, vec!["Baz", "b"]);

    let package = g
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::Package))
        .expect("package emitted");
    assert_eq!(package.id.as_str(), "a::b");
    assert_eq!(package.module_path, mods(&["a"]));

    // Exactly one edge: package `a::b` → `c::Baz`. The Foo→Bar edge
    // collapsed to a self-edge and vanished.
    assert_eq!(g.edges.len(), 1, "edges: {:?}", g.edges);
    assert_eq!(g.edges[0].from.as_str(), "a::b");
    assert_eq!(g.edges[0].to.as_str(), "c::Baz");
}

#[test]
fn module_filter_nested_collapse_outer_wins() {
    let mut g = Graph::default();
    g.add_node(node("Deep", NodeKind::Struct, mods(&["a", "b", "c"])));

    let filter = ModuleFilter::new(FilterSpec {
        collapse: vec![pat("a::b::**"), pat("a::b::c::**")],
        ..FilterSpec::default()
    });
    filter.enrich(&mut g);

    // Only the outer package `a::b`.
    assert_eq!(g.nodes.len(), 1);
    assert_eq!(g.nodes[0].id.as_str(), "a::b");
}

#[test]
fn module_filter_empty_collapse_root_synthesizes_placeholder() {
    let mut g = Graph::default();
    g.add_node(node("Other", NodeKind::Struct, mods(&["other"])));

    let filter = ModuleFilter::new(FilterSpec {
        collapse: vec![pat("missing::mod")],
        ..FilterSpec::default()
    });
    filter.enrich(&mut g);

    let placeholder = g
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::Package))
        .expect("placeholder package emitted for literal collapse pattern");
    assert_eq!(placeholder.id.as_str(), "missing::mod");
}

#[test]
fn module_filter_literal_collapse_matches_descendants() {
    // Regression: `--collapse a` should collapse `a::b::Foo` too, not
    // just an exact `a` module.
    let mut g = Graph::default();
    g.add_node(node("Foo", NodeKind::Struct, mods(&["a", "b"])));
    g.add_node(node("Bar", NodeKind::Struct, mods(&["a", "b", "c"])));

    let filter = ModuleFilter::new(FilterSpec {
        collapse: vec![pat("a")],
        ..FilterSpec::default()
    });
    filter.enrich(&mut g);

    // Single collapsed package `a`; no source-level classes remain.
    assert_eq!(g.nodes.len(), 1);
    assert!(matches!(g.nodes[0].kind, NodeKind::Package));
    assert_eq!(g.nodes[0].id.as_str(), "a");
}

#[test]
fn module_filter_collapse_depth_collapses_deeper_modules() {
    let mut g = Graph::default();
    g.add_node(node("Foo", NodeKind::Struct, mods(&["a"])));
    g.add_node(node("Bar", NodeKind::Struct, mods(&["a", "b"])));
    g.add_node(node("Baz", NodeKind::Struct, mods(&["a", "b", "c"])));
    g.add_node(node("Qux", NodeKind::Struct, mods(&["d", "e"])));

    let filter = ModuleFilter::new(FilterSpec {
        collapse_depth: Some(1),
        ..FilterSpec::default()
    });
    filter.enrich(&mut g);

    // Foo (depth 1) survives. Bar, Baz collapse into `a`. Qux collapses
    // into `d`. Two synthesized packages.
    let mut ids: Vec<_> = g.nodes.iter().map(|n| n.id.as_str().to_string()).collect();
    ids.sort();
    assert_eq!(ids, vec!["a", "a::Foo", "d"]);
}

#[test]
fn module_filter_collapse_depth_zero_collapses_all_deeper() {
    let mut g = Graph::default();
    g.add_node(node("Foo", NodeKind::Struct, mods(&["a", "b"])));
    g.add_node(node("Bar", NodeKind::Struct, mods(&["c"])));

    let filter = ModuleFilter::new(FilterSpec {
        collapse_depth: Some(0),
        ..FilterSpec::default()
    });
    filter.enrich(&mut g);

    // Every non-root module collapses to <root>.
    assert_eq!(g.nodes.len(), 1);
    assert_eq!(g.nodes[0].id.as_str(), "<root>");
}

#[test]
fn module_filter_depth_and_pattern_take_shorter_root() {
    let mut g = Graph::default();
    g.add_node(node("Foo", NodeKind::Struct, mods(&["a", "b", "c"])));

    // Pattern would collapse at `a::b` (len=2). Depth caps at 1.
    // Shorter root (depth=1 → `a`) wins.
    let filter = ModuleFilter::new(FilterSpec {
        collapse: vec![pat("a::b")],
        collapse_depth: Some(1),
        ..FilterSpec::default()
    });
    filter.enrich(&mut g);

    assert_eq!(g.nodes.len(), 1);
    assert_eq!(g.nodes[0].id.as_str(), "a");
}
