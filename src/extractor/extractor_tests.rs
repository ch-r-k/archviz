use syn::visit::Visit;

use crate::Extractor;
use crate::model::*;

fn extract(src: &str) -> Graph {
    let file = syn::parse_file(src).unwrap();

    let mut extractor = Extractor::new();
    extractor.visit_file(&file);

    extractor.graph
}

#[test]
fn extracts_structs() {
    let graph = extract(
        r#"
        struct User;
        struct Account;
    "#,
    );

    assert_eq!(graph.nodes.len(), 2);

    assert!(graph.nodes.contains(&Node {
        name: "User".into(),
        kind: NodeKind::Struct,
        module: Vec::new()
    }));

    assert!(graph.nodes.contains(&Node {
        name: "Account".into(),
        kind: NodeKind::Struct,
        module: Vec::new()
    }));
}

#[test]
fn extracts_nested_modules() {
    let graph = extract(
        r#"
        mod parser {
            pub mod ast {
                pub struct Node;
            }
        }
    "#,
    );

    assert!(graph.nodes.contains(&Node {
        name: "Node".into(),
        kind: NodeKind::Struct,
        module: vec!["parser".into(), "ast".into(),],
    }));
}

#[test]
fn extracts_traits() {
    let graph = extract(
        r#"
        trait Displayable {}
        trait Serializable {}
    "#,
    );

    assert!(graph.nodes.contains(&Node {
        name: "Displayable".into(),
        kind: NodeKind::Trait,
        module: Vec::new()
    }));

    assert!(graph.nodes.contains(&Node {
        name: "Serializable".into(),
        kind: NodeKind::Trait,
        module: Vec::new()
    }));
}

#[test]
fn extracts_trait_implementations() {
    let graph = extract(
        r#"
        trait Drawable {}

        struct Circle;

        impl Drawable for Circle {}
    "#,
    );

    assert!(graph.edges.contains(&Edge {
        from: "Circle".into(),
        to: "Drawable".into(),
        relation: Relation::Implements,
    }));

    assert!(!graph.edges.contains(&Edge {
        from: "Circle".into(),
        to: "Drawable".into(),
        relation: Relation::Composition,
    }));
}

#[test]
fn extracts_composition() {
    let graph = extract(
        r#"
        struct Engine;

        struct Car {
            engine: Engine,
        }
    "#,
    );

    assert!(graph.edges.contains(&Edge {
        from: "Car".into(),
        to: "Engine".into(),
        relation: Relation::Composition,
    }));

    assert!(!graph.edges.contains(&Edge {
        from: "Car".into(),
        to: "Engine".into(),
        relation: Relation::Implements,
    }));
}

#[test]
fn extracts_multiple_compositions() {
    let graph = extract(
        r#"
        struct Engine;
        struct Wheels;

        struct Car {
            engine: Engine,
            wheels: Wheels,
        }
    "#,
    );

    assert_eq!(graph.edges.len(), 2);
}
