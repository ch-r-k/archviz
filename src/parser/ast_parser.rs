use syn::File;

use crate::error::ArchError;
use crate::parser::Parser;
use crate::project::SourceFile;

pub struct ParsedModule {
    pub module_path: Vec<String>,
    pub ast: File,
}

pub struct AstParser;

impl Parser for AstParser {
    fn parse(&self, file: SourceFile) -> Result<ParsedModule, ArchError> {
        let ast = syn::parse_file(&file.source)?;

        Ok(ParsedModule {
            ast,
            module_path: file.module_path,
        })
    }
}
