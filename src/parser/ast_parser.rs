use anyhow::Result;
use syn::File;

use crate::project::SourceFile;
use crate::parser::Parser;

pub struct ParsedModule {
    pub module_path: Vec<String>,
    pub ast: File,
}

pub struct AstParser;

impl Parser for AstParser {
    fn parse(&self, file: SourceFile) -> Result<ParsedModule> {
        let ast = syn::parse_file(&file.source)?;

        Ok(ParsedModule {
            ast,
            module_path: file.module_path,
        })
    }
}
