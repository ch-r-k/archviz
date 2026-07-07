use crate::model::{Edge, Node, NodeKind, Relation, TypeExpr};
use crate::renderer::{DrawNode, DrawEdge};
use crate::renderer::plantuml::PlantUmlRenderer;

#[test]
fn draw_struct_node_with_packages() {
    let renderer = PlantUmlRenderer;

    let package_name_1 = "outer";
    let package_name_2 = "inner";
    let class_name = "my_class";

    let node = Node {
        name: class_name.into(),
        kind: NodeKind::Struct,
        module_path: vec![package_name_1.into(), package_name_2.into()],
    };

    assert_eq!(
        renderer.draw_node(&node),
        format!(
            "package \"{}\" {{\npackage \"{}\" {{\nclass {}\n}}\n\n}}\n\n",
            package_name_1, package_name_2, class_name
        )
    );
}

#[test]
fn draw_trait_node() {
    let renderer = PlantUmlRenderer;

    let trait_name = "my_trait";

    let node = Node {
        name: trait_name.into(),
        kind: NodeKind::Trait,
        module_path: vec![],
    };

    assert_eq!(
        renderer.draw_node(&node),
        format!("interface {}\n", trait_name)
    );
}

#[test]
fn draw_impl_node() {
    let renderer = PlantUmlRenderer;

    let trait_name = "my_trait";

    let node = Node {
        name: trait_name.into(),
        kind: NodeKind::Trait,
        module_path: vec![],
    };

    assert_eq!(
        renderer.draw_node(&node),
        format!("interface {}\n", trait_name)
    );
}

#[test]
fn draw_implement_node() {
    let renderer = PlantUmlRenderer;

    let trait_name = "my_trait";
    let impl_name = "my_impl";

    let node = Node {
        name: impl_name.into(),
        kind: NodeKind::Impl {
            trait_name: Some(trait_name.into()),
        },
        module_path: vec![],
    };

    assert_eq!(
        renderer.draw_node(&node),
        format!("class {} < {} >\n", impl_name, trait_name)
    );
}

#[test]
fn draw_enum_node() {
    let renderer = PlantUmlRenderer;

    let enum_name = "my_enum";

    let node = Node {
        name: enum_name.into(),
        kind: NodeKind::Enum,
        module_path: vec![],
    };

    assert_eq!(renderer.draw_node(&node), format!("enum {}\n", enum_name));
}

#[test]
fn draw_type_alias_node() {
    let renderer = PlantUmlRenderer;

    let alias_name = "MyAlias";

    let node = Node {
        name: alias_name.into(),
        kind: NodeKind::TypeAlias,
        module_path: vec![],
    };

    assert_eq!(
        renderer.draw_node(&node),
        format!("class {} < type >\n", alias_name)
    );
}

#[test]
fn draw_synthetic_node_with_expr() {
    let renderer = PlantUmlRenderer;

    let synthetic_name = "Vec<String>";

    let node = Node {
        name: synthetic_name.into(),
        kind: NodeKind::Synthetic {
            expr: Some(TypeExpr::Generic {
                base: "Vec".to_string(),
                args: vec![TypeExpr::Simple("String".to_string())],
            }),
        },
        module_path: vec![],
    };

    assert_eq!(
        renderer.draw_node(&node),
        format!("class \"Vec<String>\"\nnote right of \"Vec<String>\" : Generic Vec<<T>> with T = String\n")
    );
}

#[test]
fn draw_synthetic_node_without_expr() {
    let renderer = PlantUmlRenderer;

    let synthetic_name = "SomeType";

    let node = Node {
        name: synthetic_name.into(),
        kind: NodeKind::Synthetic { expr: None },
        module_path: vec![],
    };

    assert_eq!(renderer.draw_node(&node), format!("class {} \n", synthetic_name));
}

#[test]
fn draw_composition_edge() {
    let renderer = PlantUmlRenderer;

    let edge = Edge {
        from: "User".to_string(),
        to: TypeExpr::Simple("Profile".to_string()),
        relation: Relation::Composition,
    };

    assert_eq!(
        renderer.draw_edge(&edge),
        "User --> Profile : contains\n".to_string()
    );
}

#[test]
fn draw_composition_edge_with_generic_type() {
    let renderer = PlantUmlRenderer;

    let edge = Edge {
        from: "Repository".to_string(),
        to: TypeExpr::Generic {
            base: "Vec".to_string(),
            args: vec![TypeExpr::Simple("Item".to_string())],
        },
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
        from: "MyStruct".to_string(),
        to: TypeExpr::Simple("Debug".to_string()),
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
        from: "Vec<String>".to_string(),
        to: TypeExpr::Simple("Vec<T>".to_string()),
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
        name: "Vec<String>".into(),
        kind: NodeKind::Struct,
        module_path: vec![],
    };

    assert_eq!(
        renderer.draw_node(&node),
        "class \"Vec<String>\"\n".to_string()
    );
}
