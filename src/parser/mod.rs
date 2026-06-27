pub mod ast_parser;
pub mod visitor;

pub use ast_parser::*;

use anyhow::Result;
use crate::project::SourceFile;

pub trait Parser {
    fn parse(&self, file: SourceFile) -> Result<ParsedModule>;
}
