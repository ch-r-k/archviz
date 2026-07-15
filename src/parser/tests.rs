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
