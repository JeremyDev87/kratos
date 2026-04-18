use std::collections::{BTreeMap, BTreeSet};

use oxc_ast::ast::{
    ExportNamedDeclaration, IdentifierReference, ImportDeclaration, ImportDefaultSpecifier,
    ImportNamespaceSpecifier, ImportSpecifier, JSXIdentifier, Program,
};
use oxc_ast_visit::{walk, Visit};
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use std::cell::Cell;

use crate::error::KratosResult;
use crate::model::{ImportKind, ImportRecord, ImportSpecifierKind, UnusedImportRecord};

pub fn detect_unused_imports(
    program: &Program<'_>,
    imports: &[ImportRecord],
) -> KratosResult<Vec<UnusedImportRecord>> {
    let mut detector = UsageCollector::new(imports);
    detector.visit_program(program);

    let mut unused = Vec::new();

    for entry in imports {
        if is_reexport_kind(&entry.kind) {
            continue;
        }

        for specifier in &entry.specifiers {
            let Some(local) = &specifier.local else {
                continue;
            };
            let Some(imported) = &specifier.imported else {
                continue;
            };

            if specifier.kind == ImportSpecifierKind::Unknown {
                continue;
            }

            if !detector.used.contains(local) {
                unused.push(UnusedImportRecord {
                    source: entry.source.clone(),
                    local: local.clone(),
                    imported: imported.clone(),
                });
            }
        }
    }

    Ok(unused)
}

struct UsageCollector {
    watched: BTreeSet<String>,
    used: BTreeSet<String>,
    scopes: Vec<BTreeSet<String>>,
    shadowed: BTreeMap<String, usize>,
}

impl UsageCollector {
    fn new(imports: &[ImportRecord]) -> Self {
        let watched = imports
            .iter()
            .filter(|entry| !is_reexport_kind(&entry.kind))
            .flat_map(|entry| entry.specifiers.iter())
            .filter_map(|specifier| specifier.local.clone())
            .collect::<BTreeSet<_>>();

        Self {
            watched,
            used: BTreeSet::new(),
            scopes: Vec::new(),
            shadowed: BTreeMap::new(),
        }
    }

    fn mark_used(&mut self, name: &str) {
        if self.watched.contains(name) && self.shadowed.get(name).copied().unwrap_or_default() == 0
        {
            self.used.insert(name.to_string());
        }
    }

    fn register_shadow(&mut self, name: &str) {
        if !self.watched.contains(name) {
            return;
        }

        if self.scopes.len() <= 1 {
            return;
        }

        let Some(scope) = self.scopes.last_mut() else {
            return;
        };

        if scope.insert(name.to_string()) {
            *self.shadowed.entry(name.to_string()).or_default() += 1;
        }
    }
}

impl<'a> Visit<'a> for UsageCollector {
    fn enter_scope(&mut self, _flags: ScopeFlags, _scope_id: &Cell<Option<ScopeId>>) {
        self.scopes.push(BTreeSet::new());
    }

    fn leave_scope(&mut self) {
        let Some(scope) = self.scopes.pop() else {
            return;
        };

        for name in scope {
            if let Some(count) = self.shadowed.get_mut(&name) {
                *count -= 1;
                if *count == 0 {
                    self.shadowed.remove(&name);
                }
            }
        }
    }

    fn visit_import_declaration(&mut self, _declaration: &ImportDeclaration<'a>) {}

    fn visit_import_specifier(&mut self, _specifier: &ImportSpecifier<'a>) {}

    fn visit_import_default_specifier(&mut self, _specifier: &ImportDefaultSpecifier<'a>) {}

    fn visit_import_namespace_specifier(&mut self, _specifier: &ImportNamespaceSpecifier<'a>) {}

    fn visit_binding_identifier(&mut self, identifier: &oxc_ast::ast::BindingIdentifier<'a>) {
        self.register_shadow(identifier.name.as_str());
    }

    fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
        self.mark_used(identifier.name.as_str());
    }

    fn visit_jsx_identifier(&mut self, identifier: &JSXIdentifier<'a>) {
        let name = identifier.name.as_str();
        let is_intrinsic = name
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_lowercase());

        if !is_intrinsic {
            self.mark_used(name);
        }
    }

    fn visit_export_named_declaration(&mut self, declaration: &ExportNamedDeclaration<'a>) {
        if declaration.source.is_some() {
            return;
        }

        for specifier in &declaration.specifiers {
            if let Some(name) = specifier.local.identifier_name() {
                self.mark_used(name.as_str());
            }
        }

        walk::walk_export_named_declaration(self, declaration);
    }
}

fn is_reexport_kind(kind: &ImportKind) -> bool {
    matches!(
        kind,
        ImportKind::Reexport | ImportKind::ReexportAll | ImportKind::ReexportNamespace
    )
}
