/// Stable identifier for a [`Node`].
///
/// Source-level types get a fully-qualified path built from their
/// module (`"a::b::Foo"`), so two identically-named items in different
/// modules stay distinct. Synthetic types generated during graph
/// construction (compound types, std/external stubs) use a bare name
/// (`"Vec<String>"`, `"String"`) so parse-time edge targets naturally
/// match them without needing further resolution.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(pub String);

impl NodeId {
    /// Fully-qualifies `name` with `module_path` (`"a::b::Foo"`). An
    /// empty `module_path` yields a bare id.
    pub fn from_parts(module_path: &[String], name: &str) -> Self {
        if module_path.is_empty() {
            NodeId(name.to_string())
        } else {
            NodeId(format!("{}::{}", module_path.join("::"), name))
        }
    }

    /// Uses `name` verbatim (no module prefix). Intended for synthetic
    /// nodes and unresolved edge targets.
    pub fn bare(name: &str) -> Self {
        NodeId(name.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// `true` when the id has no `::` segment, i.e. it is not
    /// module-qualified.
    pub fn is_bare(&self) -> bool {
        !self.0.contains("::")
    }

    /// Everything before the trailing `::name` segment, if any.
    /// `"a::b::Foo"` → `Some("a::b")`, `"Foo"` → `None`.
    pub fn module_prefix(&self) -> Option<&str> {
        self.0.rsplit_once("::").map(|(m, _)| m)
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum NodeKind {
    Struct,
    Trait,
    Enum,
    /// `impl` block (with or without a trait). Reserved for future use — the
    /// current parser emits [`Relation::Implements`] edges rather than nodes.
    Impl {
        trait_name: Option<String>,
    },
    /// `type Foo = ...` alias. Reserved for future use — not yet emitted.
    TypeAlias,
    /// A synthetic node generated for a compound type (e.g. `Vec<String>`)
    /// that has no corresponding source-level definition. `params` carries
    /// the pre-rendered stereotype content (e.g. `"String"` for `Vec<String>`,
    /// `"A, B"` for `(A, B)`, `"T"` for the generic base `Vec<T>`); `None`
    /// renders as a plain class.
    Synthetic {
        params: Option<String>,
    },
    /// A synthetic "collapsed" module: the renderer emits an empty
    /// `package` block instead of a class. Produced by
    /// [`crate::enricher::module_filter::ModuleFilter`] when a module
    /// matches a `--collapse` pattern; all descendant nodes are dropped
    /// and their edges redirected to this node's [`NodeId`].
    Package,
}

#[derive(Debug, Clone)]
pub struct Node {
    /// Stable identifier — unique within a [`Graph`]. Used as the key
    /// on both ends of every [`Edge`].
    pub id: NodeId,
    /// Short human-readable label (`"Foo"`), rendered inside the
    /// diagram; may collide with other nodes' display names.
    pub display_name: String,
    pub kind: NodeKind,
    pub module_path: Vec<String>,
}
