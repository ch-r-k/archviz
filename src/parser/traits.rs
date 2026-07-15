use crate::error::ArchError;
use crate::parser::ast_parser::ParsedModule;
use crate::project::SourceFile;

/// Turns a [`SourceFile`] into a [`ParsedModule`]. Implementations
/// choose the concrete AST representation (currently `syn::File` via
/// [`crate::parser::ast_parser::AstParser`]).
pub trait Parser {
    fn parse(&self, file: SourceFile) -> Result<ParsedModule, ArchError>;
}
