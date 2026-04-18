use oxc_ast::ast::{
    AssignmentExpression, BindingPattern, ExportAllDeclaration, ExportNamedDeclaration, Expression,
    ExpressionStatement, ImportDeclaration, ImportDeclarationSpecifier, ImportExpression,
    ImportOrExportKind, ModuleExportName, Program, PropertyKey, TSImportEqualsDeclaration,
    TSModuleReference, VariableDeclarator,
};
use oxc_ast_visit::{walk, Visit};

use crate::error::KratosResult;
use crate::model::{ImportKind, ImportRecord, ImportSpecifier, ImportSpecifierKind};

pub fn collect_imports(program: &Program<'_>) -> KratosResult<Vec<ImportRecord>> {
    let mut collector = ImportCollector::default();
    collector.visit_program(program);
    Ok(collector.imports)
}

pub fn make_unknown_import(source: impl Into<String>) -> ImportRecord {
    ImportRecord {
        source: source.into(),
        kind: ImportKind::Unknown,
        specifiers: vec![ImportSpecifier::unknown()],
    }
}

#[derive(Default)]
struct ImportCollector {
    imports: Vec<ImportRecord>,
}

impl<'a> Visit<'a> for ImportCollector {
    fn visit_import_declaration(&mut self, declaration: &ImportDeclaration<'a>) {
        let source = declaration.source.value.as_str().to_string();
        let specifiers = declaration
            .specifiers
            .as_ref()
            .map(|specifiers| {
                specifiers
                    .iter()
                    .map(convert_import_specifier)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let kind = if specifiers.is_empty() {
            ImportKind::SideEffect
        } else {
            ImportKind::Static
        };

        self.imports.push(ImportRecord {
            source,
            kind,
            specifiers,
        });
    }

    fn visit_export_named_declaration(&mut self, declaration: &ExportNamedDeclaration<'a>) {
        if let Some(source) = &declaration.source {
            let specifiers = declaration
                .specifiers
                .iter()
                .map(|specifier| {
                    let imported = module_export_name_to_string(&specifier.local);
                    let local = module_export_name_to_string(&specifier.exported);
                    let kind = if imported.as_deref() == Some("default") {
                        ImportSpecifierKind::Default
                    } else {
                        ImportSpecifierKind::Named
                    };

                    ImportSpecifier {
                        kind,
                        imported,
                        local,
                    }
                })
                .collect::<Vec<_>>();

            self.imports.push(ImportRecord {
                source: source.value.as_str().to_string(),
                kind: ImportKind::Reexport,
                specifiers,
            });
        }

        walk::walk_export_named_declaration(self, declaration);
    }

    fn visit_export_all_declaration(&mut self, declaration: &ExportAllDeclaration<'a>) {
        let (kind, specifiers) = if let Some(exported) = &declaration.exported {
            (
                ImportKind::ReexportNamespace,
                vec![ImportSpecifier {
                    kind: ImportSpecifierKind::Namespace,
                    imported: Some("*".to_string()),
                    local: module_export_name_to_string(exported),
                }],
            )
        } else {
            (
                ImportKind::ReexportAll,
                vec![ImportSpecifier {
                    kind: ImportSpecifierKind::Unknown,
                    imported: Some("*".to_string()),
                    local: Some("*".to_string()),
                }],
            )
        };

        self.imports.push(ImportRecord {
            source: declaration.source.value.as_str().to_string(),
            kind,
            specifiers,
        });
    }

    fn visit_import_expression(&mut self, expression: &ImportExpression<'a>) {
        if let Expression::StringLiteral(literal) = &expression.source {
            self.imports.push(ImportRecord {
                source: literal.value.as_str().to_string(),
                kind: ImportKind::Dynamic,
                specifiers: Vec::new(),
            });
        }

        walk::walk_import_expression(self, expression);
    }

    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        if let Some(source) = extract_require_source(declarator.init.as_ref()) {
            self.imports
                .push(extract_require_import(&declarator.id, &source));
            return;
        }

        walk::walk_variable_declarator(self, declarator);
    }

    fn visit_expression_statement(&mut self, statement: &ExpressionStatement<'a>) {
        if let Some(source) = extract_require_source(Some(&statement.expression)) {
            self.imports.push(ImportRecord {
                source,
                kind: ImportKind::Require,
                specifiers: Vec::new(),
            });
        }

        walk::walk_expression_statement(self, statement);
    }

    fn visit_assignment_expression(&mut self, expression: &AssignmentExpression<'a>) {
        if let Some(source) = extract_require_source(Some(&expression.right)) {
            self.imports.push(ImportRecord {
                source,
                kind: ImportKind::Require,
                specifiers: Vec::new(),
            });
        }

        walk::walk_assignment_expression(self, expression);
    }

    fn visit_ts_import_equals_declaration(&mut self, declaration: &TSImportEqualsDeclaration<'a>) {
        if declaration.import_kind != ImportOrExportKind::Value {
            return;
        }

        let TSModuleReference::ExternalModuleReference(reference) = &declaration.module_reference
        else {
            return;
        };

        self.imports.push(make_unknown_binding_import(
            reference.expression.value.as_str(),
            declaration.id.name.as_str(),
        ));
    }
}

fn convert_import_specifier(specifier: &ImportDeclarationSpecifier<'_>) -> ImportSpecifier {
    match specifier {
        ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier) => ImportSpecifier {
            kind: ImportSpecifierKind::Default,
            imported: Some("default".to_string()),
            local: Some(specifier.local.name.as_str().to_string()),
        },
        ImportDeclarationSpecifier::ImportNamespaceSpecifier(specifier) => ImportSpecifier {
            kind: ImportSpecifierKind::Namespace,
            imported: Some("*".to_string()),
            local: Some(specifier.local.name.as_str().to_string()),
        },
        ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
            let imported = module_export_name_to_string(&specifier.imported)
                .unwrap_or_else(|| specifier.local.name.as_str().to_string());
            ImportSpecifier {
                kind: if imported == "default" {
                    ImportSpecifierKind::Default
                } else {
                    ImportSpecifierKind::Named
                },
                imported: Some(imported),
                local: Some(specifier.local.name.as_str().to_string()),
            }
        }
    }
}

fn module_export_name_to_string(name: &ModuleExportName<'_>) -> Option<String> {
    match name {
        ModuleExportName::IdentifierName(identifier) => Some(identifier.name.as_str().to_string()),
        ModuleExportName::IdentifierReference(identifier) => {
            Some(identifier.name.as_str().to_string())
        }
        ModuleExportName::StringLiteral(literal) => Some(literal.value.as_str().to_string()),
    }
}

fn property_key_to_string(key: &PropertyKey<'_>) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(identifier) => Some(identifier.name.as_str().to_string()),
        PropertyKey::StringLiteral(literal) => Some(literal.value.as_str().to_string()),
        _ => None,
    }
}

fn extract_require_source(expression: Option<&Expression<'_>>) -> Option<String> {
    let expression = expression?.without_parentheses();

    if expression.is_require_call() {
        let Expression::CallExpression(call) = expression else {
            return None;
        };

        let argument = call.arguments.first()?;
        let argument = argument.as_expression()?;
        let Expression::StringLiteral(literal) = argument else {
            return None;
        };

        return Some(literal.value.as_str().to_string());
    }

    match expression {
        Expression::StaticMemberExpression(member) => extract_require_source(Some(&member.object)),
        Expression::ComputedMemberExpression(member) => {
            extract_require_source(Some(&member.object))
        }
        _ => None,
    }
}

fn extract_require_import(pattern: &BindingPattern<'_>, source: &str) -> ImportRecord {
    match pattern {
        BindingPattern::BindingIdentifier(identifier) => {
            make_unknown_binding_import(source, identifier.name.as_str())
        }
        BindingPattern::ObjectPattern(object_pattern) => {
            if object_pattern.rest.is_some() {
                return make_unknown_import(source);
            }

            let mut specifiers = Vec::new();

            for property in &object_pattern.properties {
                if property.computed {
                    return make_unknown_import(source);
                }

                let Some(imported) = property_key_to_string(&property.key) else {
                    return make_unknown_import(source);
                };

                match &property.value {
                    BindingPattern::BindingIdentifier(identifier) => {
                        specifiers.push(ImportSpecifier {
                            kind: ImportSpecifierKind::Named,
                            imported: Some(imported),
                            local: Some(identifier.name.as_str().to_string()),
                        });
                    }
                    BindingPattern::AssignmentPattern(pattern) => {
                        let Some(identifier) = pattern.left.get_binding_identifier() else {
                            return make_unknown_import(source);
                        };

                        specifiers.push(ImportSpecifier {
                            kind: ImportSpecifierKind::Named,
                            imported: Some(imported),
                            local: Some(identifier.name.as_str().to_string()),
                        });
                    }
                    BindingPattern::ObjectPattern(_) | BindingPattern::ArrayPattern(_) => {
                        return make_unknown_import(source);
                    }
                }
            }

            ImportRecord {
                source: source.to_string(),
                kind: ImportKind::Require,
                specifiers,
            }
        }
        BindingPattern::ArrayPattern(_) | BindingPattern::AssignmentPattern(_) => {
            make_unknown_import(source)
        }
    }
}

fn make_unknown_binding_import(source: &str, local: &str) -> ImportRecord {
    ImportRecord {
        source: source.to_string(),
        kind: ImportKind::Require,
        specifiers: vec![ImportSpecifier {
            kind: ImportSpecifierKind::Unknown,
            imported: None,
            local: Some(local.to_string()),
        }],
    }
}
