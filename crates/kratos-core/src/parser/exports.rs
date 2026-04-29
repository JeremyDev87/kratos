use oxc_ast::ast::{
    AssignmentExpression, AssignmentTarget, ComputedMemberExpression, Declaration,
    ExportAllDeclaration, ExportDefaultDeclaration, ExportNamedDeclaration, Expression,
    ModuleExportName, Program, PropertyKey, PropertyKind, Statement, StaticMemberExpression,
    TSExportAssignment, TSNamespaceExportDeclaration,
};
use oxc_ast_visit::{walk, Visit};
use oxc_syntax::operator::AssignmentOperator;

use crate::error::KratosResult;
use crate::model::{ExportKind, ExportRecord};

pub fn collect_exports(program: &Program<'_>) -> KratosResult<Vec<ExportRecord>> {
    let mut collector = ExportCollector::default();
    collector.visit_program(program);
    Ok(collector.exports)
}

pub fn is_pure_reexport_barrel(program: &Program<'_>) -> bool {
    let mut has_reexport = false;

    for statement in &program.body {
        match statement {
            Statement::ExportNamedDeclaration(declaration)
                if declaration.source.is_some() && declaration.declaration.is_none() =>
            {
                has_reexport = true;
            }
            Statement::ExportAllDeclaration(_) => {
                has_reexport = true;
            }
            _ => return false,
        }
    }

    has_reexport
}

pub fn make_default_export() -> ExportRecord {
    ExportRecord {
        name: "default".to_string(),
        kind: ExportKind::Default,
    }
}

#[derive(Default)]
struct ExportCollector {
    exports: Vec<ExportRecord>,
}

impl<'a> Visit<'a> for ExportCollector {
    fn visit_export_default_declaration(&mut self, declaration: &ExportDefaultDeclaration<'a>) {
        self.push_default();

        walk::walk_export_default_declaration(self, declaration);
    }

    fn visit_export_named_declaration(&mut self, declaration: &ExportNamedDeclaration<'a>) {
        if let Some(inner) = &declaration.declaration {
            self.collect_declaration_exports(inner);
        } else if declaration.source.is_some() {
            for specifier in &declaration.specifiers {
                if let Some(name) = module_export_name_to_string(&specifier.exported) {
                    self.push_export(ExportRecord {
                        name,
                        kind: ExportKind::Reexport,
                    });
                }
            }
        } else {
            for specifier in &declaration.specifiers {
                if let Some(name) = module_export_name_to_string(&specifier.exported) {
                    self.push_named(&name);
                }
            }
        }

        walk::walk_export_named_declaration(self, declaration);
    }

    fn visit_export_all_declaration(&mut self, declaration: &ExportAllDeclaration<'a>) {
        if let Some(exported) = &declaration.exported {
            if let Some(name) = module_export_name_to_string(exported) {
                self.push_export(ExportRecord {
                    name,
                    kind: ExportKind::ReexportNamespace,
                });
            }
        } else {
            self.push_export(ExportRecord {
                name: "*".to_string(),
                kind: ExportKind::ReexportAll,
            });
        }
    }

    fn visit_assignment_expression(&mut self, expression: &AssignmentExpression<'a>) {
        if let Some(name) = match_commonjs_named_export(expression) {
            self.push_named(&name);
        } else if is_module_exports_assignment(expression) {
            self.push_default();

            for name in extract_module_exports_object_names(&expression.right) {
                self.push_named(&name);
            }
        }

        walk::walk_assignment_expression(self, expression);
    }

    fn visit_ts_export_assignment(&mut self, declaration: &TSExportAssignment<'a>) {
        self.push_default();
        walk::walk_ts_export_assignment(self, declaration);
    }

    fn visit_ts_namespace_export_declaration(
        &mut self,
        declaration: &TSNamespaceExportDeclaration<'a>,
    ) {
        self.push_export(ExportRecord {
            name: declaration.id.name.as_str().to_string(),
            kind: ExportKind::ReexportNamespace,
        });
        walk::walk_ts_namespace_export_declaration(self, declaration);
    }
}

impl ExportCollector {
    fn push_default(&mut self) {
        self.push_export(make_default_export());
    }

    fn push_export(&mut self, export: ExportRecord) {
        if !self.exports.contains(&export) {
            self.exports.push(export);
        }
    }

    fn push_named(&mut self, name: &str) {
        self.push_export(ExportRecord {
            name: name.to_string(),
            kind: ExportKind::Named,
        });
    }

    fn collect_declaration_exports(&mut self, declaration: &Declaration<'_>) {
        match declaration {
            Declaration::FunctionDeclaration(function) => {
                if let Some(identifier) = &function.id {
                    self.push_named(identifier.name.as_str());
                }
            }
            Declaration::ClassDeclaration(class) => {
                if let Some(identifier) = &class.id {
                    self.push_named(identifier.name.as_str());
                }
            }
            Declaration::VariableDeclaration(variable) => {
                for declarator in &variable.declarations {
                    for identifier in declarator.id.get_binding_identifiers() {
                        self.push_named(identifier.name.as_str());
                    }
                }
            }
            Declaration::TSTypeAliasDeclaration(alias) => {
                self.push_named(alias.id.name.as_str());
            }
            Declaration::TSInterfaceDeclaration(interface) => {
                self.push_named(interface.id.name.as_str());
            }
            Declaration::TSEnumDeclaration(enum_declaration) => {
                self.push_named(enum_declaration.id.name.as_str());
            }
            Declaration::TSImportEqualsDeclaration(import_equals) => {
                self.push_named(import_equals.id.name.as_str());
            }
            _ => {}
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

fn match_commonjs_named_export(expression: &AssignmentExpression<'_>) -> Option<String> {
    if expression.operator != AssignmentOperator::Assign {
        return None;
    }

    match &expression.left {
        AssignmentTarget::StaticMemberExpression(member) => {
            match_commonjs_static_named_export(member)
        }
        AssignmentTarget::ComputedMemberExpression(member) => {
            match_commonjs_computed_named_export(member)
        }
        _ => None,
    }
}

fn is_module_exports_assignment(expression: &AssignmentExpression<'_>) -> bool {
    if expression.operator != AssignmentOperator::Assign {
        return false;
    }

    let AssignmentTarget::StaticMemberExpression(member) = &expression.left else {
        return false;
    };
    let Expression::Identifier(ident) = &member.object else {
        return false;
    };

    ident.name.as_str() == "module" && member.property.name.as_str() == "exports"
}

fn extract_module_exports_object_names(expression: &Expression<'_>) -> Vec<String> {
    let Expression::ObjectExpression(object) = expression.without_parentheses() else {
        return Vec::new();
    };

    let mut names = Vec::new();

    for property in &object.properties {
        let Some(property) = property.as_property() else {
            continue;
        };

        if property.computed || property.kind != PropertyKind::Init {
            continue;
        }

        let Some(name) = property_key_to_string(&property.key) else {
            continue;
        };

        if !names.contains(&name) {
            names.push(name);
        }
    }

    names
}

fn match_commonjs_static_named_export(member: &StaticMemberExpression<'_>) -> Option<String> {
    if !member.object.is_specific_id("exports")
        && !member.object.is_specific_member_access("module", "exports")
    {
        return None;
    }

    Some(member.property.name.as_str().to_string())
}

fn match_commonjs_computed_named_export(member: &ComputedMemberExpression<'_>) -> Option<String> {
    if !member.object.is_specific_id("exports")
        && !member.object.is_specific_member_access("module", "exports")
    {
        return None;
    }

    member
        .static_property_name()
        .map(|name| name.as_str().to_string())
}
