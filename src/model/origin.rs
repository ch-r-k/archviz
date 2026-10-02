use std::collections::HashSet;
use std::sync::OnceLock;

/// The final path segment of a (possibly qualified) type name:
/// `"anyhow::Error"` → `"Error"`, `"String"` → `"String"`.
pub fn base_name(name: &str) -> &str {
    name.rsplit("::").next().unwrap_or(name)
}

/// The first path segment of a (possibly qualified) type name:
/// `"std::collections::HashMap"` → `"std"`, `"String"` → `"String"`.
pub fn first_segment(name: &str) -> &str {
    name.split("::").next().unwrap_or(name)
}

/// `true` for the standard-library crate roots a qualified path can start
/// with (`std::…`, `core::…`, `alloc::…`).
pub fn is_lang_root(segment: &str) -> bool {
    segment == "std" || segment == "core" || segment == "alloc"
}

/// Names considered part of the Rust standard library or built-in primitives.
///
/// Accepts both a bare base name (`"String"`) and a fully-qualified path
/// (`"std::collections::HashMap"`), classifying the latter by its leading
/// crate root. Anything else falls through to `External`.
pub fn is_std_name(name: &str) -> bool {
    if name.contains("::") {
        // A qualified path is std only when rooted at a language crate;
        // `anyhow::Error` must not match just because `Error` is listed.
        return is_lang_root(first_segment(name));
    }
    std_name_set().contains(name)
}

/// Returns `true` for identifiers that look like generic type parameters
/// rather than real types (`T`, `E`, `K`, `V`, `R`, `T1`, …). Both the
/// parser (when emitting composition edges to field types) and the
/// enricher (when materializing external stubs) use this to avoid
/// polluting the graph with placeholder nodes. Qualified names are judged
/// by their final segment.
pub fn looks_like_type_param(name: &str) -> bool {
    let mut chars = base_name(name).chars();
    match (chars.next(), chars.next(), chars.next()) {
        (Some(c), None, _) => c.is_ascii_uppercase(),
        (Some(c), Some(d), None) => c.is_ascii_uppercase() && d.is_ascii_digit(),
        _ => false,
    }
}

fn std_name_set() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| {
        let mut s = HashSet::new();
        s.extend(STD_TYPES);
        s.extend(PRIMITIVES);
        s
    })
}

const STD_TYPES: &[&str] = &[
    // alloc / collections
    "Vec", "String", "Box", "VecDeque", "LinkedList",
    "HashMap", "BTreeMap", "HashSet", "BTreeSet",
    // core option/result
    "Option", "Result",
    // smart pointers / sync
    "Rc", "Arc", "Weak", "RefCell", "Cell", "Mutex", "RwLock",
    "OnceCell", "OnceLock",
    // borrow
    "Cow", "Borrow", "BorrowMut",
    // ffi / path / io
    "CString", "CStr", "OsString", "OsStr", "PathBuf", "Path",
    // iterator / range
    "Range", "RangeInclusive", "Iterator", "IntoIterator",
    // error / marker
    "Error", "Debug", "Display", "Clone", "Copy", "Default",
    "Send", "Sync", "Sized", "Drop",
    // time
    "Duration", "Instant",
];

const PRIMITIVES: &[&str] = &[
    "bool", "char", "str",
    "u8", "u16", "u32", "u64", "u128", "usize",
    "i8", "i16", "i32", "i64", "i128", "isize",
    "f32", "f64", "()", "tuple", "_",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_name_strips_path() {
        assert_eq!(base_name("anyhow::Error"), "Error");
        assert_eq!(base_name("std::collections::HashMap"), "HashMap");
        assert_eq!(base_name("String"), "String");
    }

    #[test]
    fn first_segment_strips_path() {
        assert_eq!(first_segment("anyhow::Error"), "anyhow");
        assert_eq!(first_segment("std::collections::HashMap"), "std");
        assert_eq!(first_segment("String"), "String");
    }

    #[test]
    fn lang_roots_recognized() {
        assert!(is_lang_root("std"));
        assert!(is_lang_root("core"));
        assert!(is_lang_root("alloc"));
        assert!(!is_lang_root("anyhow"));
    }

    #[test]
    fn std_name_accepts_bare_and_qualified() {
        assert!(is_std_name("String"));
        assert!(is_std_name("std::collections::HashMap"));
        assert!(is_std_name("core::fmt::Debug"));
        assert!(!is_std_name("anyhow::Error"), "qualified external name must not match std");
        assert!(!is_std_name("Serde"));
    }
}
