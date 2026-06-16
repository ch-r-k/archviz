use anyhow::Result;
use syn::File;

use crate::model::ModulePath;
use crate::project::SourceFile;

pub struct ParsedModule {
    pub module: ModulePath,
    pub ast: File,
}

pub struct AstParser;

impl AstParser {
    pub fn parse(file: SourceFile) -> Result<ParsedModule> {
        let ast = syn::parse_file(&file.source)?;

        Ok(ParsedModule {
            module: file.module,
            ast,
        })
    }
}
