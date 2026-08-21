#![cfg(test)]

use crate::model::node::NodeId;
use crate::model::{Edge, Graph, Node, NodeKind, Relation};
use crate::renderer::plantuml::PlantUmlRenderer;
use crate::renderer::{DrawEdge, DrawNode, Renderer};

fn node_bare(name: &str, kind: NodeKind, module: Vec<String>) -> Node {
    Node {
        id: NodeId::from_parts(&module, name),
        display_name: name.into(),
        kind,
        module_path: module,
    }
}

#[test]
fn draw_struct_node_with_packages() {
    let renderer = PlantUmlRenderer;
    let mut graph = Graph::default();
    graph.add_node(node_bare(
        "my_class",
        NodeKind::Struct,
        vec!["outer".into(), "inner".into()],
    ));

    let out = renderer.render(&graph);
    let expected_inner = "class \"my_class\" as outer__inner__my_class\n";
    assert!(
        out.contains(&format!(
            "package \"outer\" {{\npackage \"inner\" {{\n{}}}\n\n}}\n\n",
            expected_inner
        )),
        "unexpected render output:\n{}",
        out
    );
}

#[test]
fn groups_multiple_nodes_in_same_module() {
    let renderer = PlantUmlRenderer;
    let mut graph = Graph::default();
    graph.add_node(node_bare("A", NodeKind::Struct, vec!["m".into()]));
    graph.add_node(node_bare("B", NodeKind::Struct, vec!["m".into()]));

    let out = renderer.render(&graph);
    let opens = out.matches("package \"m\" {").count();
    assert_eq!(opens, 1, "expected one package block, got {opens} in:\n{out}");
    assert!(out.contains("class \"A\" as m__A\n"));
    assert!(out.contains("class \"B\" as m__B\n"));
}

#[test]
fn draw_trait_node() {
    let renderer = PlantUmlRenderer;
    let node = node_bare("my_trait", NodeKind::Trait, vec![]);
    assert_eq!(renderer.draw_node(&node), "interface my_trait\n");
}

#[test]
fn draw_implement_node() {
    let renderer = PlantUmlRenderer;
    let node = Node {
        id: NodeId::bare("my_impl"),
        display_name: "my_impl".into(),
        kind: NodeKind::Impl {
            trait_name: Some("my_trait".into()),
        },
        module_path: vec![],
    };
    assert_eq!(
        renderer.draw_node(&node),
        "class my_impl < my_trait >\n".to_string()
    );
}

#[test]
fn draw_enum_node() {
    let renderer = PlantUmlRenderer;
    let node = node_bare("my_enum", NodeKind::Enum, vec![]);
    assert_eq!(renderer.draw_node(&node), "enum my_enum\n");
}

#[test]
fn draw_type_alias_node() {
    let renderer = PlantUmlRenderer;
    let node = node_bare("MyAlias", NodeKind::TypeAlias, vec![]);
    assert_eq!(
        renderer.draw_node(&node),
        "class MyAlias < type >\n".to_string()
    );
}

#[test]
fn draw_synthetic_node_with_expr() {
    let renderer = PlantUmlRenderer;
    let node = Node {
        id: NodeId::bare("Vec<String>"),
        display_name: "Vec<String>".into(),
        kind: NodeKind::Synthetic {
            params: Some("String".to_string()),
        },
        module_path: vec![],
    };
    assert_eq!(
        renderer.draw_node(&node),
        "class \"Vec<String>\" <String>\n".to_string()
    );
}

#[test]
fn draw_synthetic_node_without_expr() {
    let renderer = PlantUmlRenderer;
    let node = Node {
        id: NodeId::bare("SomeType"),
        display_name: "SomeType".into(),
        kind: NodeKind::Synthetic { params: None },
        module_path: vec![],
    };
    assert_eq!(renderer.draw_node(&node), "class SomeType\n".to_string());
}

#[test]
fn draw_composition_edge() {
    let renderer = PlantUmlRenderer;
    let edge = Edge {
        from: NodeId::bare("User"),
        to: NodeId::bare("Profile"),
        relation: Relation::Composition,
    };
    assert_eq!(
        renderer.draw_edge(&edge),
        "User --> Profile : contains\n".to_string()
    );
}

#[test]
fn draw_composition_edge_with_fq_ids() {
    let renderer = PlantUmlRenderer;
    let edge = Edge {
        from: NodeId("m1::A".into()),
        to: NodeId("m2::B".into()),
        relation: Relation::Composition,
    };
    assert_eq!(
        renderer.draw_edge(&edge),
        "m1__A --> m2__B : contains\n".to_string()
    );
}

#[test]
fn draw_composition_edge_with_generic_type() {
    let renderer = PlantUmlRenderer;
    let edge = Edge {
        from: NodeId::bare("Repository"),
        to: NodeId::bare("Vec<Item>"),
        relation: Relation::Composition,
    };
    assert_eq!(
        renderer.draw_edge(&edge),
        "Repository --> \"Vec<Item>\" : contains\n".to_string()
    );
}

#[test]
fn draw_implements_edge() {
    let renderer = PlantUmlRenderer;
    let edge = Edge {
        from: NodeId::bare("MyStruct"),
        to: NodeId::bare("Debug"),
        relation: Relation::Implements,
    };
    assert_eq!(
        renderer.draw_edge(&edge),
        "MyStruct ..|> Debug\n".to_string()
    );
}

#[test]
fn draw_specializes_edge() {
    let renderer = PlantUmlRenderer;
    let edge = Edge {
        from: NodeId::bare("Vec<String>"),
        to: NodeId::bare("Vec<T>"),
        relation: Relation::Specializes,
    };
    assert_eq!(
        renderer.draw_edge(&edge),
        "\"Vec<String>\" <|-- \"Vec<T>\"\n".to_string()
    );
}

#[test]
fn draw_node_with_special_chars_in_name() {
    let renderer = PlantUmlRenderer;
    let node = Node {
        id: NodeId::bare("Vec<String>"),
        display_name: "Vec<String>".into(),
        kind: NodeKind::Struct,
        module_path: vec![],
    };
    assert_eq!(
        renderer.draw_node(&node),
        "class \"Vec<String>\"\n".to_string()
    );
}

#[test]
fn draw_package_node_emits_empty_package_block() {
    let renderer = PlantUmlRenderer;
    let node = Node {
        id: NodeId::bare("a::b"),
        display_name: "b".into(),
        kind: NodeKind::Package,
        module_path: vec!["a".into()],
    };
    assert_eq!(
        renderer.draw_node(&node),
        "package \"b\" as a__b {\n}\n\n".to_string()
    );
}

#[test]
fn package_node_nests_inside_parent_module() {
    let renderer = PlantUmlRenderer;
    let mut graph = Graph::default();
    graph.add_node(Node {
        id: NodeId::bare("a::b"),
        display_name: "b".into(),
        kind: NodeKind::Package,
        module_path: vec!["a".into()],
    });

    let out = renderer.render(&graph);
    assert!(
        out.contains("package \"a\" {\npackage \"b\" as a__b {\n}\n\n}\n\n"),
        "unexpected render output:\n{}",
        out
    );
}
