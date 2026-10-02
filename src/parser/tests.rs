#![cfg(test)]

use crate::parser::type_expr::TypeExpr;
use crate::parser::type_extractor::TypeExtractor;

fn extract(src: &str) -> TypeExpr {
    let ty: syn::Type = syn::parse_str(src).expect("parse type");
    TypeExtractor::new().extract(&ty).expect("extract")
}

#[test]
fn extracts_simple_path() {
    let e = extract("String");
    assert_eq!(e.type_name(), "String");
}

#[test]
fn extracts_reference_strips_to_inner() {
    let e = extract("&str");
    assert_eq!(e.resolve_refs().type_name(), "str");
}

#[test]
fn extracts_mut_reference() {
    let e = extract("&mut Vec<u8>");
    assert_eq!(e.resolve_refs().type_name(), "Vec<u8>");
}

#[test]
fn extracts_generic_with_multiple_args() {
    let e = extract("HashMap<String, Vec<u8>>");
    assert_eq!(e.type_name(), "HashMap<String, Vec<u8>>");
    assert!(e.is_compound());
}

#[test]
fn extracts_tuple() {
    let e = extract("(A, B, C)");
    assert_eq!(e.type_name(), "(A, B, C)");
}

#[test]
fn extracts_slice_and_array() {
    assert_eq!(extract("[u8]").type_name(), "[u8]");
    assert_eq!(extract("[i32; 4]").type_name(), "[i32; N]");
}

#[test]
fn extracts_dyn_trait() {
    let e = extract("dyn Iterator + Send");
    match &e {
        TypeExpr::DynTrait(traits) => {
            assert_eq!(traits, &vec!["Iterator".to_string(), "Send".to_string()]);
        }
        other => panic!("expected DynTrait, got {:?}", other),
    }
}

#[test]
fn extracts_impl_trait() {
    let e = extract("impl Debug");
    match &e {
        TypeExpr::ImplTrait(traits) => {
            assert_eq!(traits, &vec!["Debug".to_string()]);
        }
        other => panic!("expected ImplTrait, got {:?}", other),
    }
}

#[test]
fn trait_object_names_unwrap_box_dyn() {
    let e = extract("Box<dyn Repository>");
    let names = e.trait_object_names().expect("wrapped trait");
    assert_eq!(names, vec!["Repository".to_string()]);
}

#[test]
fn compound_generic_lives_in_owner_module() {
    // Compound types live in the module of the type that uses them,
    // e.g. `Vec<Node>` used by `Graph` lands in `some::module`.
    use crate::model::{Graph, NodeKind};
    use crate::parser::graph_builder::GraphBuilder;
    use crate::parser::type_expr::TypeExpr;

    let mut g = Graph::default();
    let owner_module = vec!["some".into(), "module".into()];
    {
        let mut b = GraphBuilder::new(&mut g, &owner_module);
        b.add_struct("Owner");
        b.add_field_type(
            "Owner",
            &TypeExpr::Generic {
                base: "Vec".into(),
                args: vec![TypeExpr::Simple("String".into())],
            },
            &[],
        );
    }
    let vec_node = g
        .nodes
        .iter()
        .find(|n| n.display_name == "Vec<String>")
        .expect("Vec<String> synthesized");
    assert!(
        matches!(vec_node.kind, NodeKind::Synthetic { .. }),
        "expected synthetic kind"
    );
    assert_eq!(
        vec_node.module_path,
        vec!["some".to_string(), "module".to_string()],
        "compound must live in the owner module, got {:?}",
        vec_node.module_path
    );
}

#[test]
fn field_type_skips_declared_type_params() {
    use crate::model::Graph;
    use crate::parser::graph_builder::GraphBuilder;

    let mut g = Graph::default();
    let module = vec!["m".into()];
    {
        let mut b = GraphBuilder::new(&mut g, &module);
        b.add_struct("Blinky");
        b.add_field_type("Blinky", &TypeExpr::Simple("UiG".into()), &["UiG".into()]);
        b.add_field_type("Blinky", &TypeExpr::Simple("ConsoleUi".into()), &["UiG".into()]);
    }

    assert!(
        g.edges.iter().all(|e| e.to.as_str() != "UiG"),
        "declared type param must not become an edge target"
    );
    assert!(
        g.nodes.iter().all(|n| n.display_name != "UiG"),
        "declared type param must not become a node"
    );
    assert!(
        g.edges.iter().any(|e| e.to.as_str() == "ConsoleUi"),
        "real field types must still be recorded"
    );
}

#[test]
fn local_generic_field_links_base_and_args_and_keeps_compound() {
    use crate::model::Graph;
    use crate::parser::graph_builder::GraphBuilder;

    let mut g = Graph::default();
    let module = vec!["app".into()];
    {
        let mut b = GraphBuilder::new(&mut g, &module);
        b.add_struct("Manager");
        b.add_field_type(
            "Manager",
            &TypeExpr::Generic {
                base: "Blinky".into(),
                args: vec![TypeExpr::Simple("ConsoleUi".into())],
            },
            &[],
        );
    }

    // Concrete generic and its abstract base stay visible.
    assert!(
        g.nodes.iter().any(|n| n.display_name == "Blinky<ConsoleUi>"),
        "concrete generic compound must still be synthesized"
    );
    let base = g
        .nodes
        .iter()
        .find(|n| n.display_name == "Blinky<T>")
        .expect("generic base synthesized");
    assert_eq!(
        base.module_path,
        vec!["app".to_string()],
        "generic base must live in the owner module, got {:?}",
        base.module_path
    );

    // The owner composes the compound, the base and the type argument.
    for expected in ["Blinky<ConsoleUi>", "Blinky", "ConsoleUi"] {
        assert!(
            g.edges.iter().any(|e| e.to.as_str() == expected),
            "expected composition edge to `{expected}`"
        );
    }
}
