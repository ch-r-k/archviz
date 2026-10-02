//! Lexical resolution of bare type names using the module index.

use crate::model::origin::looks_like_type_param;
use crate::resolution::{CrateIndex, ModuleIndex};

/// Result of resolving a bare type name in a given scope.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// Resolved to a local definition (fully-qualified module path + name).
    Local(Vec<String>),
    /// Resolved to a std/core/alloc type (qualified path like `std::collections::HashMap`).
    Std(Vec<String>),
    /// Resolved to an external crate type.
    External(Vec<String>),
    /// Could not be resolved.
    Unresolved,
}

/// Resolves a bare type name `name` as seen from `owner_module`.
///
/// Resolution order:
/// 1. Type parameter / `_` → skip (Unresolved, caller should drop).
/// 2. Matching `use` import in `owner_module` (or its ancestors) → full path.
/// 2b. `use crate::...` / `use self::...` / `use super::...` handled.
/// 3. Definition `owner_module::name` exists → local.
/// 4. Unique definition `name` anywhere in project → local.
/// 5. Qualified path first segment is std/core/alloc → Std.
/// 6. Qualified path first segment is known external crate → External.
/// 7. Unresolved.
pub fn resolve_bare(
    name: &str,
    owner_module: &[String],
    module_index: &ModuleIndex,
    _crate_index: Option<&CrateIndex>,
) -> Resolution {
    // 1. Type parameter or underscore → skip.
    if looks_like_type_param(name) || name == "_" {
        return Resolution::Unresolved;
    }

    // 2. Check imports in owner_module and its ancestors.
    // Walk up the module hierarchy (including self) to find imports.
    for len in (0..=owner_module.len()).rev() {
        let scope = &owner_module[..len];
        if let Some(imports) = module_index.imports.get(scope) {
            for import in imports {
                if import.is_glob {
                    // Glob import: if the path prefix + name matches a definition,
                    // we could resolve it. For now, skip glob resolution (too ambiguous).
                    continue;
                }
                let import_name = import.alias.as_deref().unwrap_or_else(|| {
                    import.path.last().map(|s| s.as_str()).unwrap_or("")
                });
                if import_name == name {
                    // Found a matching import. The import path is absolute.
                    // If the import path starts with `crate::`, `self::`, or `super::`,
                    // normalize it relative to the owner module.
                    let mut full_path = import.path.clone();
                    if let Some(first) = full_path.first() {
                        match first.as_str() {
                            "crate" => {
                                // `crate::` refers to crate root — strip it.
                                full_path.remove(0);
                            }
                            "self" => {
                                // `self::` refers to the current module.
                                if full_path.len() > 1 {
                                    let mut new_prefix = owner_module.to_vec();
                                    new_prefix.extend(full_path[1..].iter().cloned());
                                    full_path = new_prefix;
                                } else {
                                    full_path = owner_module.to_vec();
                                }
                            }
                            "super" => {
                                // `super::` refers to parent module.
                                if !owner_module.is_empty() {
                                    if full_path.len() > 1 {
                                        let mut new_prefix = owner_module[..owner_module.len() - 1].to_vec();
                                        new_prefix.extend(full_path[1..].iter().cloned());
                                        full_path = new_prefix;
                                    } else {
                                        full_path = owner_module[..owner_module.len() - 1].to_vec();
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    // If the resolved path actually names a definition, use it.
                    // Otherwise fall through to the unique-definition lookup (which
                    // will find the canonical definition, e.g. through re-exports).
                    let candidate_path = full_path.clone();
                    if candidate_path.len() >= 1 {
                        let candidate_name = &candidate_path[candidate_path.len() - 1];
                        let candidate_mod = &candidate_path[..candidate_path.len() - 1];
                        if module_index.definitions.contains_key(candidate_name) {
                            if module_index.definitions[candidate_name]
                                .iter()
                                .any(|d| d.module_path == candidate_mod)
                            {
                                return Resolution::Local(candidate_path);
                            }
                        }
                    }
                    // If the import path doesn't match a real definition, fall through
                    // to the unique-definition lookup below.
                }
            }
        }
    }

    // 3. Check if `owner_module::name` is defined.
    let mut fq_path = owner_module.to_vec();
    fq_path.push(name.to_string());
    if module_index.definitions.contains_key(name) {
        for def in &module_index.definitions[name] {
            if def.module_path == fq_path {
                return Resolution::Local(fq_path);
            }
        }
    }

    // 4. Unique definition anywhere in project with this base name.
    if let Some(defs) = module_index.definitions.get(name) {
        if defs.len() == 1 {
            let mut full_path = defs[0].module_path.clone();
            full_path.push(name.to_string());
            return Resolution::Local(full_path);
        }
    }

    // 5/6. Check if the name is a qualified path (contains `::`).
    // Wait — this function receives a *bare* name. Qualified paths are handled
    // earlier (they contain `::` and go through path-based classification).
    // So this function only handles bare names. The qualified path handling
    // happens in OriginResolver before calling here.
    // But: some qualified paths may have been left bare if they came from
    // imports? Actually, the parser now preserves full path spelling, so
    // qualified paths arrive at OriginResolver with `::`. They don't reach here.
    // This function is only for bare names.

    // However, we still need to check if the bare name matches a known
    // external crate name (unlikely — bare names are usually local).
    // The crate_index is for first-segment matching of qualified paths.

    // 7. Unresolved.
    Resolution::Unresolved
}

/// Tries to resolve a qualified path (containing `::`) using imports/definitions.
/// Returns the canonical fully-qualified path if found, else the original path.
#[allow(dead_code)]
pub fn resolve_qualified(
    path: &[String],
    owner_module: &[String],
    _module_index: &ModuleIndex,
) -> Vec<String> {
    let first = &path[0];

    // Handle `crate::`, `self::`, `super::` prefixes.
    if first == "crate" {
        // Replace `crate` with the crate root (empty module path for crate root).
        if path.len() == 1 {
            return Vec::new(); // just `crate` → crate root (unusual in type position)
        }
        let mut resolved = Vec::new();
        resolved.extend(path[1..].iter().cloned());
        return resolved;
    }

    if first == "self" {
        // Relative to owner_module.
        if path.len() == 1 {
            return owner_module.to_vec();
        }
        let mut resolved = owner_module.to_vec();
        resolved.extend(path[1..].iter().cloned());
        return resolved;
    }

    if first == "super" {
        // Relative to parent of owner_module.
        if owner_module.is_empty() {
            return path.to_vec(); // no parent
        }
        let mut resolved = owner_module[..owner_module.len() - 1].to_vec();
        if path.len() == 1 {
            return resolved; // just `super` → parent module
        }
        resolved.extend(path[1..].iter().cloned());
        return resolved;
    }

    // Check imports for a matching path prefix (e.g., `use foo::bar; bar::Baz`).
    // This is complex — for now, just return the path as-is if it's already
    // absolute-looking, or if it matches a definition.
    path.to_vec()
}