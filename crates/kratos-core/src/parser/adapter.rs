use std::path::Path;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

use crate::error::KratosResult;

use super::exports;
use super::imports;
use super::unused_imports;
use super::ParsedModule;

pub fn parse_module_source(path: &Path, source: &str) -> KratosResult<ParsedModule> {
    let source_type =
        SourceType::from_path(path).unwrap_or_else(|_| SourceType::default().with_module(true));
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();

    if parsed.panicked {
        return Ok(ParsedModule::default());
    }

    if !parsed.errors.is_empty() {
        return Ok(ParsedModule::default());
    }

    let imports = imports::collect_imports(&parsed.program)?;
    let exports = exports::collect_exports(&parsed.program)?;
    let unused_imports = unused_imports::detect_unused_imports(&parsed.program, &imports)?;
    let is_pure_reexport_barrel = exports::is_pure_reexport_barrel(&parsed.program);

    Ok(ParsedModule {
        imports,
        exports,
        unused_imports,
        is_pure_reexport_barrel,
    })
}
