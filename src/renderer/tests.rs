use crate::model::{Edge, Node, NodeKind, Relation, TypeExpr};
use crate::renderer::DrawNode;
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
