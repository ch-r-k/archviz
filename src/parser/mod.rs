pub mod ast_parser;
pub mod graph_builder;
pub mod type_expr;
pub mod type_extractor;
pub mod visitor;
mod tests;

pub use ast_parser::*;

use crate::error::ArchError;
use crate::project::SourceFile;

pub trait Parser {
    fn parse(&self, file: SourceFile) -> Result<ParsedModule, ArchError>;
}
