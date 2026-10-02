//! Builds a lexical index from the loaded source files:
//! - a module tree (from file paths)
//! - a definition index: base name → list of (module_path, kind)
//! - an import table: module_path → list of imports (use items)

use std::collections::{HashMap, HashSet};

use syn::visit::Visit;

use crate::model::node::NodeKind;
use crate::project::SourceFile;

/// Index of all source-level definitions and imports across the project.
#[derive(Debug, Default)]
pub struct ModuleIndex {
    /// All module paths that exist in the project (from file paths).
    pub modules: HashSet<Vec<String>>,
    /// Definition index: base name → list of (module_path, kind).
    pub definitions: HashMap<String, Vec<Definition>>,
    /// Import table: module_path → list of imports visible in that module.
    pub imports: HashMap<Vec<String>, Vec<Import>>,
}

/// A source-level definition (struct, trait, enum, type alias).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Definition {
    pub module_path: Vec<String>,
    pub kind: DefinitionKind,
}

/// Kind of a definition for display/debug purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefinitionKind {
    Struct,
    Trait,
    Enum,
    TypeAlias,
}

impl DefinitionKind {
    #[allow(dead_code)]
    fn from_node_kind(k: NodeKind) -> Option<Self> {
        match k {
            NodeKind::Struct => Some(DefinitionKind::Struct),
            NodeKind::Trait => Some(DefinitionKind::Trait),
            NodeKind::Enum => Some(DefinitionKind::Enum),
            NodeKind::TypeAlias => Some(DefinitionKind::TypeAlias),
            _ => None,
        }
    }
}

/// A single `use` item import, normalized to an absolute path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    /// The full path being imported (e.g., "std::collections::HashMap").
    pub path: Vec<String>,
    /// The alias used locally, if any (e.g., `use foo as bar` → Some("bar")).
    /// For glob imports (`use path::*`), this is None and `is_glob` is true.
    pub alias: Option<String>,
    /// True for glob imports (`use path::*`).
    pub is_glob: bool,
}

impl ModuleIndex {
    pub fn build(files: &[SourceFile]) -> Self {
        let mut index = ModuleIndex::default();

        // First pass: collect all module paths from files.
        for file in files {
            index.modules.insert(file.module_path.clone());
        }

        // Second pass: parse each file and extract definitions + imports.
        for file in files {
            if let Ok(ast) = syn::parse_file(&file.source) {
                let mut visitor = IndexVisitor {
                    index: &mut index,
                    current_module: file.module_path.clone(),
                };
                visitor.visit_file(&ast);
            }
        }

        index
    }
}

/// AST visitor that collects definitions and imports per module.
struct IndexVisitor<'a> {
    index: &'a mut ModuleIndex,
    current_module: Vec<String>,
}

impl<'a> Visit<'_> for IndexVisitor<'a> {
    fn visit_item_struct(&mut self, node: &syn::ItemStruct) {
        let name = node.ident.to_string();
        self.index.definitions
            .entry(name)
            .or_default()
            .push(Definition {
                module_path: self.current_module.clone(),
                kind: DefinitionKind::Struct,
            });
        syn::visit::visit_item_struct(self, node);
    }

    fn visit_item_trait(&mut self, node: &syn::ItemTrait) {
        let name = node.ident.to_string();
        self.index.definitions
            .entry(name)
            .or_default()
            .push(Definition {
                module_path: self.current_module.clone(),
                kind: DefinitionKind::Trait,
            });
        syn::visit::visit_item_trait(self, node);
    }

    fn visit_item_enum(&mut self, node: &syn::ItemEnum) {
        let name = node.ident.to_string();
        self.index.definitions
            .entry(name)
            .or_default()
            .push(Definition {
                module_path: self.current_module.clone(),
                kind: DefinitionKind::Enum,
            });
        syn::visit::visit_item_enum(self, node);
    }

    fn visit_item_type(&mut self, node: &syn::ItemType) {
        let name = node.ident.to_string();
        self.index.definitions
            .entry(name)
            .or_default()
            .push(Definition {
                module_path: self.current_module.clone(),
                kind: DefinitionKind::TypeAlias,
            });
        syn::visit::visit_item_type(self, node);
    }

    fn visit_item_mod(&mut self, node: &syn::ItemMod) {
        if let Some(content) = &node.content {
            let name = node.ident.to_string();
            let child_module = self.current_module.iter().cloned().chain(std::iter::once(name)).collect();
            let old_module = std::mem::replace(&mut self.current_module, child_module);
            for item in &content.1 {
                self.visit_item(item);
            }
            self.current_module = old_module;
        } else {
            // External module (mod foo; without inline body) — its definitions
            // will come from the corresponding file's own visit.
            syn::visit::visit_item_mod(self, node);
        }
    }

    fn visit_item_use(&mut self, node: &syn::ItemUse) {
        // Normalize the use tree into absolute paths + aliases.
        let mut imports = Vec::new();
        collect_use_tree(&node.tree, &mut Vec::new(), &mut imports);

        for imp in imports {
            // Convert path segments to Vec<String>.
            let path: Vec<String> = imp.path.iter().map(|s| s.to_string()).collect();
            let import = Import {
                path,
                alias: imp.alias.map(|s| s.to_string()),
                is_glob: imp.is_glob,
            };
            self.index.imports
                .entry(self.current_module.clone())
                .or_default()
                .push(import);
        }

        syn::visit::visit_item_use(self, node);
    }
}

/// Intermediate representation of a collected import from a use tree.
struct CollectedImport {
    path: Vec<syn::Ident>,
    alias: Option<syn::Ident>,
    is_glob: bool,
}

fn collect_use_tree(tree: &syn::UseTree, prefix: &mut Vec<syn::Ident>, out: &mut Vec<CollectedImport>) {
    match tree {
        syn::UseTree::Path(p) => {
            prefix.push(p.ident.clone());
            collect_use_tree(&p.tree, prefix, out);
            prefix.pop();
        }
        syn::UseTree::Name(n) => {
            let mut path = prefix.clone();
            path.push(n.ident.clone());
            out.push(CollectedImport {
                path,
                alias: None, // syn 3: UseName has no alias; aliases are represented via UseRename
                is_glob: false,
            });
        }
        syn::UseTree::Rename(r) => {
            let mut path = prefix.clone();
            path.push(r.ident.clone());
            out.push(CollectedImport {
                path,
                alias: Some(r.rename.clone()),
                is_glob: false,
            });
        }
        syn::UseTree::Glob(_) => {
            out.push(CollectedImport {
                path: prefix.clone(),
                alias: None,
                is_glob: true,
            });
        }
        syn::UseTree::Group(g) => {
            for item in &g.items {
                collect_use_tree(item, prefix, out);
            }
        }
    }
}