use std::collections::HashSet;

/// Where a referenced type comes from, relative to the analyzed project.
///
/// Determined heuristically from the type's base name. A later pass could
/// swap this out for a rust-analyzer / rustc-driven resolver for exact
/// answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeOrigin {
    /// Defined somewhere in the project being analyzed (a node already
    /// exists in the graph with this base name).
    Local,
    /// From the Rust standard library or a language primitive.
    Std,
    /// Anything else — assumed to come from an external crate/dependency.
    External,
}

impl TypeOrigin {
    /// Module path used to place a synthetic node representing this origin
    /// inside the diagram. `Local` returns `None` because local types already
    /// have their own module path from the parser.
    pub fn module_path(&self) -> Option<Vec<String>> {
        match self {
            TypeOrigin::Local => None,
            TypeOrigin::Std => Some(vec!["std".to_string()]),
            TypeOrigin::External => Some(vec!["external".to_string()]),
        }
    }
}

/// Classifies a type base name.
///
/// * If `local_names` contains it → `Local`.
/// * Else if it is a known std/primitive name → `Std`.
/// * Else → `External`.
pub fn classify(base: &str, local_names: &HashSet<String>) -> TypeOrigin {
    if local_names.contains(base) {
        TypeOrigin::Local
    } else if is_std_name(base) {
        TypeOrigin::Std
    } else {
        TypeOrigin::External
    }
}

/// Names considered part of the Rust standard library or built-in primitives.
///
/// Kept intentionally conservative — anything not on this list falls through
/// to `External`. Extend as needed.
pub fn is_std_name(base: &str) -> bool {
    is_std_type(base) || is_primitive(base)
}

fn is_std_type(base: &str) -> bool {
    matches!(
        base,
        // alloc / collections
        "Vec" | "String" | "Box" | "VecDeque" | "LinkedList"
        | "HashMap" | "BTreeMap" | "HashSet" | "BTreeSet"
        // core option/result
        | "Option" | "Result"
        // smart pointers / sync
        | "Rc" | "Arc" | "Weak" | "RefCell" | "Cell" | "Mutex" | "RwLock"
        | "OnceCell" | "OnceLock"
        // borrow
        | "Cow" | "Borrow" | "BorrowMut"
        // ffi / path / io
        | "CString" | "CStr" | "OsString" | "OsStr" | "PathBuf" | "Path"
        // iterator / range
        | "Range" | "RangeInclusive" | "Iterator" | "IntoIterator"
        // error / marker
        | "Error" | "Debug" | "Display" | "Clone" | "Copy" | "Default"
        | "Send" | "Sync" | "Sized" | "Drop"
        // time
        | "Duration" | "Instant"
    )
}

fn is_primitive(base: &str) -> bool {
    matches!(
        base,
        "bool" | "char" | "str"
        | "u8" | "u16" | "u32" | "u64" | "u128" | "usize"
        | "i8" | "i16" | "i32" | "i64" | "i128" | "isize"
        | "f32" | "f64" | "()" | "tuple" | "_"
    )
}
