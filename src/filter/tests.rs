#![cfg(test)]

use crate::filter::pattern::ModulePattern;
use crate::filter::spec::{Decision, FilterSpec};

fn path(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

fn pat(s: &str) -> ModulePattern {
    ModulePattern::parse(s).expect("valid pattern")
}

#[test]
fn literal_pattern_exact_match_only() {
    let p = pat("a::b");
    assert!(p.matches(&path(&["a", "b"])));
    assert!(!p.matches(&path(&["a", "b", "c"])));
    assert!(!p.matches(&path(&["a"])));
    assert!(!p.matches(&path(&["x", "a", "b"])));
}

#[test]
fn single_star_matches_one_segment() {
    let p = pat("a::*");
    assert!(p.matches(&path(&["a", "b"])));
    assert!(p.matches(&path(&["a", "z"])));
    assert!(!p.matches(&path(&["a"])));
    assert!(!p.matches(&path(&["a", "b", "c"])));
}

#[test]
fn double_star_matches_self_and_descendants() {
    let p = pat("a::b::**");
    assert!(p.matches(&path(&["a", "b"])));
    assert!(p.matches(&path(&["a", "b", "c"])));
    assert!(p.matches(&path(&["a", "b", "c", "d"])));
    assert!(!p.matches(&path(&["a"])));
    assert!(!p.matches(&path(&["a", "c"])));
}

#[test]
fn leading_double_star_matches_any_prefix() {
    let p = pat("**::internal");
    assert!(p.matches(&path(&["internal"])));
    assert!(p.matches(&path(&["a", "internal"])));
    assert!(p.matches(&path(&["a", "b", "internal"])));
    assert!(!p.matches(&path(&["internal", "x"])));
}

#[test]
fn double_star_alone_matches_everything() {
    let p = pat("**");
    assert!(p.matches(&[]));
    assert!(p.matches(&path(&["a"])));
    assert!(p.matches(&path(&["a", "b", "c"])));
}

#[test]
fn parse_rejects_empty_and_partial_wildcards() {
    assert!(ModulePattern::parse("").is_err());
    assert!(ModulePattern::parse("a::").is_err());
    assert!(ModulePattern::parse("foo*").is_err());
    assert!(ModulePattern::parse("a::foo*::b").is_err());
}

#[test]
fn spec_exclude_wins_over_include() {
    let spec = FilterSpec {
        include: vec![pat("a::**")],
        exclude: vec![pat("a::internal::**")],
        collapse: vec![],
        collapse_depth: None,
    };
    assert_eq!(spec.decides(&path(&["a", "public"])), Decision::Keep);
    assert_eq!(spec.decides(&path(&["a", "internal"])), Decision::Drop);
    assert_eq!(spec.decides(&path(&["a", "internal", "x"])), Decision::Drop);
    assert_eq!(spec.decides(&path(&["b"])), Decision::Drop);
}

#[test]
fn spec_empty_include_keeps_everything_by_default() {
    let spec = FilterSpec::default();
    assert_eq!(spec.decides(&path(&["a", "b"])), Decision::Keep);
    assert_eq!(spec.decides(&[]), Decision::Keep);
}

#[test]
fn spec_decides_ignores_collapse() {
    // Collapse is handled by ModuleFilter, not FilterSpec::decides —
    // decides only returns Keep or Drop for exclude/include.
    let spec = FilterSpec {
        include: vec![],
        exclude: vec![pat("a::internal::**")],
        collapse: vec![pat("a::db")],
        collapse_depth: None,
    };
    assert_eq!(spec.decides(&path(&["a", "db"])), Decision::Keep);
    assert_eq!(spec.decides(&path(&["a", "internal"])), Decision::Drop);
    assert_eq!(spec.decides(&path(&["a", "public"])), Decision::Keep);
}

// ---------------------------------------------------------------
// ModuleFilter tests (moved from src/enricher/tests.rs when the
// filter became its own pipeline stage).
// ---------------------------------------------------------------

use crate::filter::module_filter::ModuleFilter;
use crate::filter::traits::Filter;
use crate::model::node::NodeId;
use crate::model::{Edge, Graph, Node, NodeKind, Relation};

fn mods(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

fn mf_node(name: &str, kind: NodeKind, module_path: Vec<String>) -> Node {
    Node {
        id: NodeId::from_parts(&module_path, name),
        display_name: name.into(),
        kind,
        module_path,
    }
}

#[test]
fn module_filter_drops_excluded_nodes_and_dangling_edges() {
    let mut g = Graph::default();
    g.add_node(mf_node("Public", NodeKind::Struct, mods(&["a", "b", "public"])));
    g.add_node(mf_node(
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
    filter.apply(&mut g);

    assert_eq!(g.nodes.len(), 1);
    assert_eq!(g.nodes[0].display_name, "Public");
    assert!(g.edges.is_empty(), "edges: {:?}", g.edges);
}

#[test]
fn module_filter_include_only_keeps_matches() {
    let mut g = Graph::default();
    g.add_node(mf_node("Foo", NodeKind::Struct, mods(&["api"])));
    g.add_node(mf_node("Bar", NodeKind::Struct, mods(&["internal"])));

    let filter = ModuleFilter::new(FilterSpec {
        include: vec![pat("api::**")],
        ..FilterSpec::default()
    });
    filter.apply(&mut g);

    let names: Vec<_> = g.nodes.iter().map(|n| n.display_name.clone()).collect();
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn module_filter_empty_spec_is_noop() {
    let mut g = Graph::default();
    g.add_node(mf_node("A", NodeKind::Struct, mods(&["m"])));
    g.edges.push(Edge {
        from: NodeId::from_parts(&mods(&["m"]), "A"),
        to: NodeId::bare("String"),
        relation: Relation::Composition,
    });

    let before_nodes = g.nodes.len();
    let before_edges = g.edges.len();
    ModuleFilter::new(FilterSpec::default()).apply(&mut g);
    assert_eq!(g.nodes.len(), before_nodes);
    assert_eq!(g.edges.len(), before_edges);
}

#[test]
fn module_filter_collapse_replaces_subtree_with_package() {
    let mut g = Graph::default();
    g.add_node(mf_node("Foo", NodeKind::Struct, mods(&["a", "b"])));
    g.add_node(mf_node("Bar", NodeKind::Struct, mods(&["a", "b"])));
    g.add_node(mf_node("Baz", NodeKind::Struct, mods(&["c"])));

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
    filter.apply(&mut g);

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

    assert_eq!(g.edges.len(), 1, "edges: {:?}", g.edges);
    assert_eq!(g.edges[0].from.as_str(), "a::b");
    assert_eq!(g.edges[0].to.as_str(), "c::Baz");
}

#[test]
fn module_filter_nested_collapse_outer_wins() {
    let mut g = Graph::default();
    g.add_node(mf_node("Deep", NodeKind::Struct, mods(&["a", "b", "c"])));

    let filter = ModuleFilter::new(FilterSpec {
        collapse: vec![pat("a::b::**"), pat("a::b::c::**")],
        ..FilterSpec::default()
    });
    filter.apply(&mut g);

    assert_eq!(g.nodes.len(), 1);
    assert_eq!(g.nodes[0].id.as_str(), "a::b");
}

#[test]
fn module_filter_empty_collapse_root_synthesizes_placeholder() {
    let mut g = Graph::default();
    g.add_node(mf_node("Other", NodeKind::Struct, mods(&["other"])));

    let filter = ModuleFilter::new(FilterSpec {
        collapse: vec![pat("missing::mod")],
        ..FilterSpec::default()
    });
    filter.apply(&mut g);

    let placeholder = g
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::Package))
        .expect("placeholder package emitted for literal collapse pattern");
    assert_eq!(placeholder.id.as_str(), "missing::mod");
}

#[test]
fn module_filter_literal_collapse_matches_descendants() {
    let mut g = Graph::default();
    g.add_node(mf_node("Foo", NodeKind::Struct, mods(&["a", "b"])));
    g.add_node(mf_node("Bar", NodeKind::Struct, mods(&["a", "b", "c"])));

    let filter = ModuleFilter::new(FilterSpec {
        collapse: vec![pat("a")],
        ..FilterSpec::default()
    });
    filter.apply(&mut g);

    assert_eq!(g.nodes.len(), 1);
    assert!(matches!(g.nodes[0].kind, NodeKind::Package));
    assert_eq!(g.nodes[0].id.as_str(), "a");
}

#[test]
fn module_filter_collapse_depth_collapses_deeper_modules() {
    let mut g = Graph::default();
    g.add_node(mf_node("Foo", NodeKind::Struct, mods(&["a"])));
    g.add_node(mf_node("Bar", NodeKind::Struct, mods(&["a", "b"])));
    g.add_node(mf_node("Baz", NodeKind::Struct, mods(&["a", "b", "c"])));
    g.add_node(mf_node("Qux", NodeKind::Struct, mods(&["d", "e"])));

    let filter = ModuleFilter::new(FilterSpec {
        collapse_depth: Some(1),
        ..FilterSpec::default()
    });
    filter.apply(&mut g);

    let mut ids: Vec<_> = g.nodes.iter().map(|n| n.id.as_str().to_string()).collect();
    ids.sort();
    assert_eq!(ids, vec!["a", "a::Foo", "d"]);
}

#[test]
fn module_filter_collapse_depth_zero_collapses_all_deeper() {
    let mut g = Graph::default();
    g.add_node(mf_node("Foo", NodeKind::Struct, mods(&["a", "b"])));
    g.add_node(mf_node("Bar", NodeKind::Struct, mods(&["c"])));

    let filter = ModuleFilter::new(FilterSpec {
        collapse_depth: Some(0),
        ..FilterSpec::default()
    });
    filter.apply(&mut g);

    assert_eq!(g.nodes.len(), 1);
    assert_eq!(g.nodes[0].id.as_str(), "<root>");
}

#[test]
fn module_filter_depth_and_pattern_take_shorter_root() {
    let mut g = Graph::default();
    g.add_node(mf_node("Foo", NodeKind::Struct, mods(&["a", "b", "c"])));

    let filter = ModuleFilter::new(FilterSpec {
        collapse: vec![pat("a::b")],
        collapse_depth: Some(1),
        ..FilterSpec::default()
    });
    filter.apply(&mut g);

    assert_eq!(g.nodes.len(), 1);
    assert_eq!(g.nodes[0].id.as_str(), "a");
}

#[test]
fn module_filter_literal_include_matches_descendants() {
    let mut g = Graph::default();
    g.add_node(mf_node("Foo", NodeKind::Struct, mods(&["a", "b"])));
    g.add_node(mf_node("Bar", NodeKind::Struct, mods(&["a", "b", "c"])));
    g.add_node(mf_node("Zoo", NodeKind::Struct, mods(&["x"])));

    let filter = ModuleFilter::new(FilterSpec {
        include: vec![pat("a")],
        ..FilterSpec::default()
    });
    filter.apply(&mut g);

    let mut names: Vec<_> = g.nodes.iter().map(|n| n.display_name.clone()).collect();
    names.sort();
    assert_eq!(names, vec!["Bar", "Foo"]);
}

#[test]
fn module_filter_include_keeps_std_stub_edge_targets() {
    let mut g = Graph::default();
    g.add_node(mf_node("Foo", NodeKind::Struct, mods(&["keep"])));
    g.add_node(Node {
        id: NodeId::bare("String"),
        display_name: "String".into(),
        kind: NodeKind::Synthetic { params: None },
        module_path: mods(&["std"]),
    });
    g.edges.push(Edge {
        from: NodeId::from_parts(&mods(&["keep"]), "Foo"),
        to: NodeId::bare("String"),
        relation: Relation::Composition,
    });

    let filter = ModuleFilter::new(FilterSpec {
        include: vec![pat("keep")],
        ..FilterSpec::default()
    });
    filter.apply(&mut g);

    assert!(
        g.nodes.iter().any(|n| n.display_name == "String"),
        "std stub was dropped"
    );
    assert_eq!(g.edges.len(), 1, "surviving edge to std stub was dropped");
}

#[test]
fn module_filter_include_prunes_unreferenced_std_stub() {
    let mut g = Graph::default();
    g.add_node(mf_node("Foo", NodeKind::Struct, mods(&["drop"])));
    g.add_node(Node {
        id: NodeId::bare("String"),
        display_name: "String".into(),
        kind: NodeKind::Synthetic { params: None },
        module_path: mods(&["std"]),
    });
    g.edges.push(Edge {
        from: NodeId::from_parts(&mods(&["drop"]), "Foo"),
        to: NodeId::bare("String"),
        relation: Relation::Composition,
    });

    let filter = ModuleFilter::new(FilterSpec {
        include: vec![pat("keep")],
        ..FilterSpec::default()
    });
    filter.apply(&mut g);

    assert!(
        g.nodes.iter().all(|n| n.display_name != "String"),
        "orphan std stub not pruned"
    );
}

#[test]
fn module_filter_include_drops_compound_types_under_excluded_owner() {
    let mut g = Graph::default();
    g.add_node(mf_node("Owner", NodeKind::Struct, mods(&["drop"])));
    g.add_node(Node {
        id: NodeId::bare("Vec<Owner>"),
        display_name: "Vec<Owner>".into(),
        kind: NodeKind::Synthetic { params: Some("Owner".into()) },
        module_path: mods(&["drop"]),
    });

    let filter = ModuleFilter::new(FilterSpec {
        include: vec![pat("keep")],
        ..FilterSpec::default()
    });
    filter.apply(&mut g);

    assert!(
        g.nodes.is_empty(),
        "compound type from excluded module leaked: {:?}",
        g.nodes.iter().map(|n| &n.id).collect::<Vec<_>>()
    );
}
