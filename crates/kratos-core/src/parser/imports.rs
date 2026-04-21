use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};

use oxc_ast::ast::{
    AssignmentExpression, BindingIdentifier, BindingPattern, BlockStatement, CallExpression,
    ExportAllDeclaration, ExportNamedDeclaration, Expression, ExpressionStatement, FunctionBody,
    Function, FunctionType, ImportDeclaration, ImportDeclarationSpecifier, ImportExpression,
    ImportOrExportKind, ModuleExportName, ObjectExpression, ObjectPropertyKind, Program,
    PropertyKey, PropertyKind, Statement, TSImportEqualsDeclaration, TSModuleReference,
    VariableDeclarator, VariableDeclarationKind,
};
use oxc_ast_visit::{walk, Visit};
use oxc_syntax::scope::{ScopeFlags, ScopeId};

use crate::error::KratosResult;
use crate::model::{ImportKind, ImportRecord, ImportSpecifier, ImportSpecifierKind};

pub fn collect_imports(program: &Program<'_>) -> KratosResult<Vec<ImportRecord>> {
    let scope_shadows = collect_shadowed_dynamic_bindings(program);
    let scope_dynamic_bindings = collect_scoped_dynamic_bindings(program, &scope_shadows);
    let scope_loader_bindings = collect_scoped_loader_bindings(program);
    let scope_then_callback_bindings = collect_scoped_then_callback_bindings(program);
    let mut collector =
        ImportCollector::new(
            scope_shadows,
            scope_dynamic_bindings,
            scope_loader_bindings,
            scope_then_callback_bindings,
        );
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
    scope_shadows: BTreeMap<usize, BTreeSet<String>>,
    scope_dynamic_bindings: BTreeMap<usize, DynamicBindings>,
    scope_loader_bindings: BTreeMap<usize, LoaderBindings>,
    scope_then_callback_bindings: BTreeMap<usize, ThenCallbackBindings>,
    next_scope_index: usize,
    active_dynamic_bindings: DynamicBindingCounts,
    dynamic_scope_stack: Vec<DynamicScopeFrame>,
    loader_scope_stack: Vec<LoaderBindings>,
    then_callback_scope_stack: Vec<ThenCallbackBindings>,
    scopes: Vec<BTreeSet<String>>,
    shadowed: BTreeMap<String, usize>,
}

impl ImportCollector {
    fn is_builtin_require_shadowed(&self) -> bool {
        self.shadowed.get("require").copied().unwrap_or_default() > 0
    }

    fn new(
        scope_shadows: BTreeMap<usize, BTreeSet<String>>,
        scope_dynamic_bindings: BTreeMap<usize, DynamicBindings>,
        scope_loader_bindings: BTreeMap<usize, LoaderBindings>,
        scope_then_callback_bindings: BTreeMap<usize, ThenCallbackBindings>,
    ) -> Self {
        Self {
            scope_shadows,
            scope_dynamic_bindings,
            scope_loader_bindings,
            scope_then_callback_bindings,
            next_scope_index: 0,
            active_dynamic_bindings: DynamicBindingCounts::default(),
            dynamic_scope_stack: Vec::new(),
            loader_scope_stack: Vec::new(),
            then_callback_scope_stack: Vec::new(),
            scopes: Vec::new(),
            shadowed: BTreeMap::new(),
            imports: Vec::new(),
        }
    }

}

#[derive(Clone, Default)]
struct DynamicBindings {
    react_objects: BTreeSet<String>,
    react_lazy_functions: BTreeSet<String>,
    next_dynamic_functions: BTreeSet<String>,
}

struct DynamicScopeFrame {
    flags: ScopeFlags,
    bindings: DynamicBindings,
}

enum DynamicBindingKind {
    ReactObject,
    ReactLazyFunction,
    NextDynamicFunction,
}

#[derive(Default)]
struct DynamicBindingCounts {
    react_objects: BTreeMap<String, usize>,
    react_lazy_functions: BTreeMap<String, usize>,
    next_dynamic_functions: BTreeMap<String, usize>,
}

impl DynamicBindingCounts {
    fn push_scope(&mut self, bindings: &DynamicBindings) {
        for name in &bindings.react_objects {
            *self.react_objects.entry(name.clone()).or_default() += 1;
        }
        for name in &bindings.react_lazy_functions {
            *self.react_lazy_functions.entry(name.clone()).or_default() += 1;
        }
        for name in &bindings.next_dynamic_functions {
            *self.next_dynamic_functions.entry(name.clone()).or_default() += 1;
        }
    }

    fn pop_scope(&mut self, bindings: &DynamicBindings) {
        for name in &bindings.react_objects {
            decrement_binding_count(&mut self.react_objects, name);
        }
        for name in &bindings.react_lazy_functions {
            decrement_binding_count(&mut self.react_lazy_functions, name);
        }
        for name in &bindings.next_dynamic_functions {
            decrement_binding_count(&mut self.next_dynamic_functions, name);
        }
    }
}

fn decrement_binding_count(counts: &mut BTreeMap<String, usize>, name: &str) {
    if let Some(count) = counts.get_mut(name) {
        *count -= 1;
        if *count == 0 {
            counts.remove(name);
        }
    }
}

#[derive(Clone, Debug)]
enum DynamicImportUsage {
    Default,
    Named(String),
}

enum ThenCallbackBinding {
    Namespace(String),
    Destructured(BTreeMap<String, String>),
}

#[derive(Clone, Debug)]
struct TrackedDynamicImport {
    source: String,
    usage: DynamicImportUsage,
}

type LoaderBindings = BTreeMap<String, TrackedDynamicImport>;
type ThenCallbackBindings = BTreeMap<String, DynamicImportUsage>;

impl<'a> Visit<'a> for ImportCollector {
    fn visit_program(&mut self, program: &Program<'a>) {
        self.register_hoisted_loader_declarations(&program.body);
        walk::walk_program(self, program);
    }

    fn enter_scope(&mut self, flags: ScopeFlags, _scope_id: &Cell<Option<ScopeId>>) {
        let dynamic_bindings = self
            .scope_dynamic_bindings
            .get(&self.next_scope_index)
            .cloned()
            .unwrap_or_default();
        let loader_bindings = self
            .scope_loader_bindings
            .get(&self.next_scope_index)
            .cloned()
            .unwrap_or_default();
        let then_callback_bindings = self
            .scope_then_callback_bindings
            .get(&self.next_scope_index)
            .cloned()
            .unwrap_or_default();
        let scope_bindings = self
            .scope_shadows
            .get(&self.next_scope_index)
            .cloned()
            .unwrap_or_default();
        self.next_scope_index += 1;

        self.active_dynamic_bindings.push_scope(&dynamic_bindings);
        self.dynamic_scope_stack.push(DynamicScopeFrame {
            flags,
            bindings: dynamic_bindings,
        });
        self.loader_scope_stack.push(loader_bindings);
        self.then_callback_scope_stack.push(then_callback_bindings);

        for name in &scope_bindings {
            *self.shadowed.entry(name.clone()).or_default() += 1;
        }

        self.scopes.push(scope_bindings);
    }

    fn leave_scope(&mut self) {
        if let Some(frame) = self.dynamic_scope_stack.pop() {
            self.active_dynamic_bindings.pop_scope(&frame.bindings);
        }
        self.loader_scope_stack.pop();
        self.then_callback_scope_stack.pop();

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

    fn visit_import_declaration(&mut self, declaration: &ImportDeclaration<'a>) {
        let source = declaration.source.value.as_str();
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
            source: source.to_string(),
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

    fn visit_call_expression(&mut self, expression: &CallExpression<'a>) {
        if let Some(dynamic_usage) = self.extract_dynamic_wrapper_usage_current(expression) {
            self.walk_dynamic_wrapper_call(expression, dynamic_usage);
            return;
        }

        walk::walk_call_expression(self, expression);
    }

    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        let mut skip_initializer_walk = false;

        if let Some(initializer) = declarator.init.as_ref() {
            if let Some(binding_kind) = self.classify_dynamic_alias_expression_current(initializer)
            {
                self.register_runtime_dynamic_alias(
                    declarator.kind,
                    &declarator.id,
                    binding_kind,
                );
            }

            if let Some(local_name) = binding_pattern_name(&declarator.id) {
                if let Some(usage) = self.extract_then_callback_usage_current(initializer) {
                    self.register_runtime_then_callback_binding(
                        declarator.kind,
                        local_name.clone(),
                        usage,
                    );
                }

                let direct_loader_binding = match initializer.without_parentheses() {
                    Expression::ArrowFunctionExpression(_)
                    | Expression::FunctionExpression(_) => {
                        self.extract_loader_expression_usage_current(initializer)
                    }
                    _ => None,
                };
                let tracked_import =
                    direct_loader_binding.clone().or_else(|| {
                        let Expression::Identifier(identifier) =
                            initializer.without_parentheses()
                        else {
                            return None;
                        };

                        self.resolve_loader_binding(identifier.name.as_str())
                    });

                if let Some(tracked_import) = tracked_import {
                    self.register_runtime_loader_binding(
                        declarator.kind,
                        local_name,
                        tracked_import,
                    );
                }

                skip_initializer_walk = direct_loader_binding.is_some();
            }
        }

        if let Some(source) = extract_require_source(
            declarator.init.as_ref(),
            self.is_builtin_require_shadowed(),
        ) {
            self.register_runtime_require_dynamic_bindings(
                declarator.kind,
                &declarator.id,
                &source,
            );
            self.imports
                .push(extract_require_import(&declarator.id, &source));
            return;
        }

        if skip_initializer_walk {
            return;
        }

        walk::walk_variable_declarator(self, declarator);
    }

    fn visit_expression_statement(&mut self, statement: &ExpressionStatement<'a>) {
        if let Some(source) = extract_require_source(
            Some(&statement.expression),
            self.is_builtin_require_shadowed(),
        ) {
            self.imports.push(ImportRecord {
                source,
                kind: ImportKind::Require,
                specifiers: Vec::new(),
            });
        }

        walk::walk_expression_statement(self, statement);
    }

    fn visit_assignment_expression(&mut self, expression: &AssignmentExpression<'a>) {
        if let Some(source) = extract_require_source(
            Some(&expression.right),
            self.is_builtin_require_shadowed(),
        ) {
            self.imports.push(ImportRecord {
                source,
                kind: ImportKind::Require,
                specifiers: Vec::new(),
            });
        }

        walk::walk_assignment_expression(self, expression);
    }

    fn visit_block_statement(&mut self, block: &BlockStatement<'a>) {
        self.register_hoisted_loader_declarations(&block.body);
        walk::walk_block_statement(self, block);
    }

    fn visit_function_body(&mut self, body: &FunctionBody<'a>) {
        self.register_hoisted_loader_declarations(&body.statements);
        walk::walk_function_body(self, body);
    }

    fn visit_function(&mut self, function: &Function<'a>, flags: ScopeFlags) {
        if matches!(
            function.r#type,
            FunctionType::FunctionDeclaration | FunctionType::TSDeclareFunction
        ) {
            if let (Some(id), Some(body)) = (&function.id, function.body.as_ref()) {
                if let Some(result) = extract_function_body_return_expression(body) {
                    if let Some(tracked_import) = extract_loader_expression_usage(result) {
                        self.register_runtime_loader_binding(
                            VariableDeclarationKind::Var,
                            id.name.as_str().to_string(),
                            tracked_import,
                        );
                        return;
                    }
                }
            }
        }

        walk::walk_function(self, function, flags);
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

impl ImportCollector {
    fn resolve_dynamic_binding_kind(&self, name: &str) -> Option<DynamicBindingKind> {
        for (scope_bindings, dynamic_frame) in self
            .scopes
            .iter()
            .zip(self.dynamic_scope_stack.iter())
            .rev()
        {
            if !scope_bindings.contains(name) {
                continue;
            }

            if dynamic_frame.bindings.react_objects.contains(name) {
                return Some(DynamicBindingKind::ReactObject);
            }
            if dynamic_frame.bindings.react_lazy_functions.contains(name) {
                return Some(DynamicBindingKind::ReactLazyFunction);
            }
            if dynamic_frame.bindings.next_dynamic_functions.contains(name) {
                return Some(DynamicBindingKind::NextDynamicFunction);
            }

            return None;
        }

        None
    }

    fn resolve_loader_binding(&self, name: &str) -> Option<TrackedDynamicImport> {
        for (scope_bindings, loader_bindings) in
            self.scopes.iter().zip(self.loader_scope_stack.iter()).rev()
        {
            if !scope_bindings.contains(name) {
                continue;
            }

            return loader_bindings.get(name).cloned();
        }

        None
    }

    fn resolve_then_callback_usage(&self, name: &str) -> Option<DynamicImportUsage> {
        for (scope_bindings, callback_bindings) in self
            .scopes
            .iter()
            .zip(self.then_callback_scope_stack.iter())
            .rev()
        {
            if !scope_bindings.contains(name) {
                continue;
            }

            return callback_bindings.get(name).cloned();
        }

        None
    }

    fn is_react_wrapper_object_current(&self, expression: &Expression<'_>) -> bool {
        match expression.without_parentheses() {
            Expression::Identifier(identifier) => matches!(
                self.resolve_dynamic_binding_kind(identifier.name.as_str()),
                Some(DynamicBindingKind::ReactObject)
            ),
            Expression::CallExpression(call) => {
                require_call_matches_source(call, "react", &self.shadowed)
            }
            Expression::StaticMemberExpression(member) => {
                member.property.name.as_str() == "default"
                    && matches!(
                        &member.object.without_parentheses(),
                        Expression::CallExpression(call)
                            if require_call_matches_source(call, "react", &self.shadowed)
                    )
            }
            _ => false,
        }
    }

    fn is_react_lazy_callee_current(&self, expression: &Expression<'_>) -> bool {
        match expression {
            Expression::StaticMemberExpression(member) => {
                member.property.name.as_str() == "lazy"
                    && self.is_react_wrapper_object_current(&member.object)
            }
            Expression::Identifier(identifier) => matches!(
                self.resolve_dynamic_binding_kind(identifier.name.as_str()),
                Some(DynamicBindingKind::ReactLazyFunction)
            ),
            _ => false,
        }
    }

    fn is_next_dynamic_callee_current(&self, expression: &Expression<'_>) -> bool {
        match expression {
            Expression::Identifier(identifier) => matches!(
                self.resolve_dynamic_binding_kind(identifier.name.as_str()),
                Some(DynamicBindingKind::NextDynamicFunction)
            ),
            Expression::StaticMemberExpression(member) => {
                member.property.name.as_str() == "default"
                    && matches!(
                        &member.object.without_parentheses(),
                        Expression::CallExpression(call)
                            if require_call_matches_source(call, "next/dynamic", &self.shadowed)
                    )
            }
            _ => false,
        }
    }

    fn classify_dynamic_alias_expression_current(
        &self,
        expression: &Expression<'_>,
    ) -> Option<DynamicBindingKind> {
        match expression.without_parentheses() {
            Expression::Identifier(identifier) => {
                self.resolve_dynamic_binding_kind(identifier.name.as_str())
            }
            Expression::StaticMemberExpression(member) => {
                if member.property.name.as_str() == "lazy"
                    && self.is_react_wrapper_object_current(&member.object)
                {
                    return Some(DynamicBindingKind::ReactLazyFunction);
                }

                if member.property.name.as_str() == "default"
                    && matches!(
                        &member.object.without_parentheses(),
                        Expression::CallExpression(call)
                            if require_call_matches_source(call, "next/dynamic", &self.shadowed)
                    )
                {
                    return Some(DynamicBindingKind::NextDynamicFunction);
                }

                None
            }
            _ => None,
        }
    }

    fn extract_loader_callback_usage_current(
        &self,
        callback: &Expression<'_>,
    ) -> Option<TrackedDynamicImport> {
        match callback {
            Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_) => {
                self.extract_loader_expression_usage_current(callback)
            }
            Expression::Identifier(identifier) => {
                self.resolve_loader_binding(identifier.name.as_str())
            }
            _ => None,
        }
    }

    fn extract_then_callback_usage_current(
        &self,
        expression: &Expression<'_>,
    ) -> Option<DynamicImportUsage> {
        match expression.without_parentheses() {
            Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_) => {
                extract_then_callback_usage(expression)
            }
            Expression::Identifier(identifier) => {
                self.resolve_then_callback_usage(identifier.name.as_str())
            }
            _ => None,
        }
    }

    fn extract_then_call_usage_current(
        &self,
        call: &CallExpression<'_>,
    ) -> Option<TrackedDynamicImport> {
        if call.arguments.is_empty() {
            return None;
        }

        let Expression::StaticMemberExpression(member) = call.callee.without_parentheses() else {
            return None;
        };

        if member.property.name.as_str() != "then" {
            return None;
        }

        let Expression::ImportExpression(import) = member.object.without_parentheses() else {
            return None;
        };
        let Expression::StringLiteral(literal) = &import.source else {
            return None;
        };

        let callback = call.arguments.first()?.as_expression()?;
        Some(TrackedDynamicImport {
            source: literal.value.as_str().to_string(),
            usage: self.extract_then_callback_usage_current(callback)?,
        })
    }

    fn extract_loader_expression_usage_current(
        &self,
        expression: &Expression<'_>,
    ) -> Option<TrackedDynamicImport> {
        let expression = expression.without_parentheses();

        match expression.without_parentheses() {
            Expression::ImportExpression(import) => {
                let Expression::StringLiteral(literal) = &import.source else {
                    return None;
                };

                Some(TrackedDynamicImport {
                    source: literal.value.as_str().to_string(),
                    usage: DynamicImportUsage::Default,
                })
            }
            Expression::ArrowFunctionExpression(arrow) => {
                let result = arrow
                    .get_expression()
                    .or_else(|| extract_function_body_return_expression(&arrow.body))?;
                self.extract_loader_expression_usage_current(result)
            }
            Expression::FunctionExpression(function) => {
                let body = function.body.as_ref()?;
                let result = extract_function_body_return_expression(body)?;
                self.extract_loader_expression_usage_current(result)
            }
            Expression::CallExpression(call) => self.extract_then_call_usage_current(call),
            _ => None,
        }
    }

    fn extract_dynamic_wrapper_usage_current(
        &self,
        call: &CallExpression<'_>,
    ) -> Option<TrackedDynamicImport> {
        let callee = call.callee.without_parentheses();
        let callback = call.arguments.first()?.as_expression()?.without_parentheses();

        if self.is_react_lazy_callee_current(callee) {
            return self.extract_loader_callback_usage_current(callback);
        }

        if self.is_next_dynamic_callee_current(callee) {
            return self.extract_loader_callback_usage_current(callback);
        }

        None
    }

    fn walk_dynamic_wrapper_call(
        &mut self,
        call: &CallExpression<'_>,
        dynamic_import: TrackedDynamicImport,
    ) {
        self.imports.push(ImportRecord {
            source: dynamic_import.source,
            kind: ImportKind::Dynamic,
            specifiers: dynamic_usage_specifiers(&dynamic_import.usage),
        });

        walk::walk_expression(self, &call.callee);

        if let Some(type_arguments) = &call.type_arguments {
            walk::walk_ts_type_parameter_instantiation(self, type_arguments);
        }

        for argument in call.arguments.iter().skip(1) {
            walk::walk_argument(self, argument);
        }
    }

    fn register_runtime_dynamic_alias(
        &mut self,
        declaration_kind: VariableDeclarationKind,
        pattern: &BindingPattern<'_>,
        binding_kind: DynamicBindingKind,
    ) {
        let Some(local_name) = binding_pattern_name(pattern) else {
            return;
        };

        let target_frame = if declaration_kind == VariableDeclarationKind::Var {
            self.dynamic_scope_stack
                .iter_mut()
                .rev()
                .find(|frame| frame.flags.is_var())
        } else {
            self.dynamic_scope_stack.last_mut()
        };

        let Some(frame) = target_frame else {
            return;
        };

        let inserted = match binding_kind {
            DynamicBindingKind::ReactObject => {
                frame.bindings.react_objects.insert(local_name.clone())
            }
            DynamicBindingKind::ReactLazyFunction => frame
                .bindings
                .react_lazy_functions
                .insert(local_name.clone()),
            DynamicBindingKind::NextDynamicFunction => frame
                .bindings
                .next_dynamic_functions
                .insert(local_name.clone()),
        };

        if !inserted {
            return;
        }

        match binding_kind {
            DynamicBindingKind::ReactObject => {
                *self
                    .active_dynamic_bindings
                    .react_objects
                    .entry(local_name)
                    .or_default() += 1;
            }
            DynamicBindingKind::ReactLazyFunction => {
                *self
                    .active_dynamic_bindings
                    .react_lazy_functions
                    .entry(local_name)
                    .or_default() += 1;
            }
            DynamicBindingKind::NextDynamicFunction => {
                *self
                    .active_dynamic_bindings
                    .next_dynamic_functions
                    .entry(local_name)
                    .or_default() += 1;
            }
        }
    }

    fn register_runtime_require_dynamic_bindings(
        &mut self,
        declaration_kind: VariableDeclarationKind,
        pattern: &BindingPattern<'_>,
        source: &str,
    ) {
        let target_frame = if declaration_kind == VariableDeclarationKind::Var {
            self.dynamic_scope_stack
                .iter_mut()
                .rev()
                .find(|frame| frame.flags.is_var())
        } else {
            self.dynamic_scope_stack.last_mut()
        };

        let Some(frame) = target_frame else {
            return;
        };

        let mut bindings = DynamicBindings::default();
        collect_require_dynamic_bindings(&mut bindings, pattern, source);

        for name in bindings.react_objects {
            if frame.bindings.react_objects.insert(name.clone()) {
                *self
                    .active_dynamic_bindings
                    .react_objects
                    .entry(name)
                    .or_default() += 1;
            }
        }

        for name in bindings.react_lazy_functions {
            if frame.bindings.react_lazy_functions.insert(name.clone()) {
                *self
                    .active_dynamic_bindings
                    .react_lazy_functions
                    .entry(name)
                    .or_default() += 1;
            }
        }

        for name in bindings.next_dynamic_functions {
            if frame.bindings.next_dynamic_functions.insert(name.clone()) {
                *self
                    .active_dynamic_bindings
                    .next_dynamic_functions
                    .entry(name)
                    .or_default() += 1;
            }
        }
    }

    fn register_runtime_loader_binding(
        &mut self,
        declaration_kind: VariableDeclarationKind,
        local_name: String,
        tracked_import: TrackedDynamicImport,
    ) {
        let target_index = if declaration_kind == VariableDeclarationKind::Var {
            self.dynamic_scope_stack
                .iter()
                .enumerate()
                .rev()
                .find(|(_, frame)| frame.flags.is_var())
                .map(|(index, _)| index)
        } else {
            self.loader_scope_stack.len().checked_sub(1)
        };

        let Some(target_index) = target_index else {
            return;
        };

        self.loader_scope_stack[target_index].insert(local_name, tracked_import);
    }

    fn register_runtime_then_callback_binding(
        &mut self,
        declaration_kind: VariableDeclarationKind,
        local_name: String,
        usage: DynamicImportUsage,
    ) {
        let target_index = if declaration_kind == VariableDeclarationKind::Var {
            self.dynamic_scope_stack
                .iter()
                .enumerate()
                .rev()
                .find(|(_, frame)| frame.flags.is_var())
                .map(|(index, _)| index)
        } else {
            self.then_callback_scope_stack.len().checked_sub(1)
        };

        let Some(target_index) = target_index else {
            return;
        };

        self.then_callback_scope_stack[target_index].insert(local_name, usage);
    }

    fn register_hoisted_loader_declarations(
        &mut self,
        statements: &[Statement<'_>],
    ) {
        for statement in statements {
            let Statement::FunctionDeclaration(function) = statement else {
                continue;
            };
            let (Some(id), Some(body)) = (&function.id, function.body.as_ref()) else {
                continue;
            };
            let Some(result) = extract_function_body_return_expression(body) else {
                continue;
            };
            let Some(tracked_import) = extract_loader_expression_usage(result) else {
                continue;
            };

            self.register_runtime_loader_binding(
                VariableDeclarationKind::Var,
                id.name.as_str().to_string(),
                tracked_import,
            );
        }
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

fn collect_dynamic_bindings(
    program: &Program<'_>,
    scope_shadows: &BTreeMap<usize, BTreeSet<String>>,
) -> DynamicBindings {
    let mut collector = DynamicBindingCollector::new(scope_shadows);
    collector.visit_program(program);
    collector.bindings
}

struct DynamicBindingCollector {
    bindings: DynamicBindings,
    scope_shadows: BTreeMap<usize, BTreeSet<String>>,
    next_scope_index: usize,
    shadowed_require_scopes: Vec<bool>,
    shadowed_require_count: usize,
}

impl DynamicBindingCollector {
    fn new(scope_shadows: &BTreeMap<usize, BTreeSet<String>>) -> Self {
        Self {
            bindings: DynamicBindings::default(),
            scope_shadows: scope_shadows.clone(),
            next_scope_index: 0,
            shadowed_require_scopes: Vec::new(),
            shadowed_require_count: 0,
        }
    }

    fn is_builtin_require_shadowed(&self) -> bool {
        self.shadowed_require_count > 0
    }
}

impl<'a> Visit<'a> for DynamicBindingCollector {
    fn enter_scope(&mut self, _flags: ScopeFlags, _scope_id: &Cell<Option<ScopeId>>) {
        let shadowed_require = self
            .scope_shadows
            .get(&self.next_scope_index)
            .is_some_and(|bindings| bindings.contains("require"));
        self.next_scope_index += 1;

        if shadowed_require {
            self.shadowed_require_count += 1;
        }

        self.shadowed_require_scopes.push(shadowed_require);
    }

    fn leave_scope(&mut self) {
        let Some(shadowed_require) = self.shadowed_require_scopes.pop() else {
            return;
        };

        if shadowed_require {
            self.shadowed_require_count -= 1;
        }
    }

    fn visit_import_declaration(&mut self, declaration: &ImportDeclaration<'a>) {
        match declaration.source.value.as_str() {
            "react" => collect_react_import_bindings(
                &mut self.bindings,
                declaration
                    .specifiers
                    .as_ref()
                    .map(|specifiers| specifiers.as_slice()),
            ),
            "next/dynamic" => collect_next_dynamic_import_bindings(
                &mut self.bindings,
                declaration
                    .specifiers
                    .as_ref()
                    .map(|specifiers| specifiers.as_slice()),
            ),
            _ => {}
        }
    }

    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        if let Some(source) = extract_require_source(
            declarator.init.as_ref(),
            self.is_builtin_require_shadowed(),
        ) {
            collect_require_dynamic_bindings(&mut self.bindings, &declarator.id, &source);
        }

        walk::walk_variable_declarator(self, declarator);
    }

    fn visit_ts_import_equals_declaration(&mut self, declaration: &TSImportEqualsDeclaration<'a>) {
        if declaration.import_kind != ImportOrExportKind::Value {
            return;
        }

        let TSModuleReference::ExternalModuleReference(reference) = &declaration.module_reference
        else {
            return;
        };

        collect_require_dynamic_binding_name(
            &mut self.bindings,
            reference.expression.value.as_str(),
            declaration.id.name.as_str(),
        );
    }
}

fn collect_scoped_dynamic_bindings(
    program: &Program<'_>,
    scope_shadows: &BTreeMap<usize, BTreeSet<String>>,
) -> BTreeMap<usize, DynamicBindings> {
    let mut collector = ScopedDynamicBindingCollector::new(scope_shadows);
    collector.visit_program(program);
    collector.bindings
}

struct ScopedDynamicBindingCollector {
    scopes: Vec<ScopeEntry>,
    bindings: BTreeMap<usize, DynamicBindings>,
    next_scope_index: usize,
}

impl ScopedDynamicBindingCollector {
    fn new(_scope_shadows: &BTreeMap<usize, BTreeSet<String>>) -> Self {
        Self {
            scopes: Vec::new(),
            bindings: BTreeMap::new(),
            next_scope_index: 0,
        }
    }
}

impl<'a> Visit<'a> for ScopedDynamicBindingCollector {
    fn enter_scope(&mut self, flags: ScopeFlags, _scope_id: &Cell<Option<ScopeId>>) {
        self.scopes.push(ScopeEntry {
            index: self.next_scope_index,
            flags,
        });
        self.next_scope_index += 1;
    }

    fn leave_scope(&mut self) {
        self.scopes.pop();
    }

    fn visit_import_declaration(&mut self, declaration: &ImportDeclaration<'a>) {
        let Some(scope) = self.scopes.last() else {
            return;
        };

        let entry = self.bindings.entry(scope.index).or_default();
        match declaration.source.value.as_str() {
            "react" => collect_react_import_bindings(
                entry,
                declaration
                    .specifiers
                    .as_ref()
                    .map(|specifiers| specifiers.as_slice()),
            ),
            "next/dynamic" => collect_next_dynamic_import_bindings(
                entry,
                declaration
                    .specifiers
                    .as_ref()
                    .map(|specifiers| specifiers.as_slice()),
            ),
            _ => {}
        }
    }

    fn visit_ts_import_equals_declaration(&mut self, declaration: &TSImportEqualsDeclaration<'a>) {
        if declaration.import_kind != ImportOrExportKind::Value {
            return;
        }

        let TSModuleReference::ExternalModuleReference(reference) = &declaration.module_reference
        else {
            return;
        };

        self.register_local_name_in_current_scope(
            reference.expression.value.as_str(),
            declaration.id.name.as_str(),
        );
    }
}

impl ScopedDynamicBindingCollector {
    fn register_local_name_in_current_scope(&mut self, source: &str, local_name: &str) {
        let Some(scope) = self.scopes.last() else {
            return;
        };

        collect_require_dynamic_binding_name(
            self.bindings.entry(scope.index).or_default(),
            source,
            local_name,
        );
    }
}

fn collect_scoped_loader_bindings(program: &Program<'_>) -> BTreeMap<usize, LoaderBindings> {
    let mut collector = ScopedLoaderBindingCollector {
        scope_shadows: collect_shadowed_dynamic_bindings(program),
        ..ScopedLoaderBindingCollector::default()
    };
    collector.visit_program(program);
    collector.bindings
}

fn collect_scoped_then_callback_bindings(
    program: &Program<'_>,
) -> BTreeMap<usize, ThenCallbackBindings> {
    let mut collector = ScopedThenCallbackCollector::default();
    collector.visit_program(program);
    collector.bindings
}

#[derive(Default)]
struct ScopedLoaderBindingCollector {
    scopes: Vec<ScopeEntry>,
    bindings: BTreeMap<usize, LoaderBindings>,
    scope_shadows: BTreeMap<usize, BTreeSet<String>>,
    next_scope_index: usize,
    active_loader_bindings: BTreeMap<String, Vec<TrackedDynamicImport>>,
    active_loader_scope_names: BTreeMap<usize, Vec<String>>,
    active_scope_bindings: Vec<BTreeSet<String>>,
}

#[derive(Default)]
struct ScopedThenCallbackCollector {
    scopes: Vec<ScopeEntry>,
    bindings: BTreeMap<usize, ThenCallbackBindings>,
    next_scope_index: usize,
}

impl ScopedLoaderBindingCollector {
    fn resolve_loader_identifier(&self, name: &str) -> Option<TrackedDynamicImport> {
        for (scope, scope_bindings) in self
            .scopes
            .iter()
            .zip(self.active_scope_bindings.iter())
            .rev()
        {
            if !scope_bindings.contains(name) {
                continue;
            }

            return self
                .bindings
                .get(&scope.index)
                .and_then(|bindings| bindings.get(name))
                .cloned();
        }

        None
    }

    fn register_in_current_scope(&mut self, name: String, tracked_import: TrackedDynamicImport) {
        let Some(scope) = self.scopes.last() else {
            return;
        };

        self.active_loader_bindings
            .entry(name.clone())
            .or_default()
            .push(tracked_import.clone());
        self.active_loader_scope_names
            .entry(scope.index)
            .or_default()
            .push(name.clone());
        self.bindings
            .entry(scope.index)
            .or_default()
            .insert(name, tracked_import);
    }

    fn register_in_nearest_var_scope(
        &mut self,
        name: String,
        tracked_import: TrackedDynamicImport,
    ) {
        let Some(scope) = self.scopes.iter().rev().find(|scope| scope.flags.is_var()) else {
            return;
        };

        self.active_loader_bindings
            .entry(name.clone())
            .or_default()
            .push(tracked_import.clone());
        self.active_loader_scope_names
            .entry(scope.index)
            .or_default()
            .push(name.clone());
        self.bindings
            .entry(scope.index)
            .or_default()
            .insert(name, tracked_import);
    }
}

impl ScopedThenCallbackCollector {
    fn register_in_nearest_var_scope(&mut self, name: String, usage: DynamicImportUsage) {
        let Some(scope) = self.scopes.iter().rev().find(|scope| scope.flags.is_var()) else {
            return;
        };

        self.bindings
            .entry(scope.index)
            .or_default()
            .insert(name, usage);
    }
}

impl<'a> Visit<'a> for ScopedLoaderBindingCollector {
    fn enter_scope(&mut self, flags: ScopeFlags, _scope_id: &Cell<Option<ScopeId>>) {
        let scope_bindings = self
            .scope_shadows
            .get(&self.next_scope_index)
            .cloned()
            .unwrap_or_default();

        self.scopes.push(ScopeEntry {
            index: self.next_scope_index,
            flags,
        });
        self.next_scope_index += 1;
        self.active_scope_bindings.push(scope_bindings);
    }

    fn leave_scope(&mut self) {
        let Some(scope) = self.scopes.pop() else {
            return;
        };

        if let Some(names) = self.active_loader_scope_names.remove(&scope.index) {
            for name in names {
                if let Some(active_bindings) = self.active_loader_bindings.get_mut(&name) {
                    active_bindings.pop();
                    if active_bindings.is_empty() {
                        self.active_loader_bindings.remove(&name);
                    }
                }
            }
        }

        let Some(scope_bindings) = self.active_scope_bindings.pop() else {
            return;
        };
        drop(scope_bindings);
    }

    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        if let (Some(local_name), Some(initializer)) =
            (binding_pattern_name(&declarator.id), declarator.init.as_ref())
        {
            let tracked_import = extract_loader_binding_usage(initializer).or_else(|| {
                let Expression::Identifier(identifier) = initializer.without_parentheses() else {
                    return None;
                };

                self.resolve_loader_identifier(identifier.name.as_str())
            });

            if let Some(tracked_import) = tracked_import {
                if declarator.kind == VariableDeclarationKind::Var {
                    self.register_in_nearest_var_scope(local_name, tracked_import);
                } else {
                    self.register_in_current_scope(local_name, tracked_import);
                }
            }
        }

        walk::walk_variable_declarator(self, declarator);
    }

    fn visit_function(&mut self, function: &Function<'a>, flags: ScopeFlags) {
        if matches!(
            function.r#type,
            FunctionType::FunctionDeclaration | FunctionType::TSDeclareFunction
        ) {
            if let (Some(id), Some(body)) = (&function.id, function.body.as_ref()) {
                if let Some(result) = extract_function_body_return_expression(body) {
                    if let Some(tracked_import) = extract_loader_expression_usage(result) {
                        self.register_in_current_scope(
                            id.name.as_str().to_string(),
                            tracked_import,
                        );
                    }
                }
            }
        }

        walk::walk_function(self, function, flags);
    }
}

impl<'a> Visit<'a> for ScopedThenCallbackCollector {
    fn enter_scope(&mut self, flags: ScopeFlags, _scope_id: &Cell<Option<ScopeId>>) {
        self.scopes.push(ScopeEntry {
            index: self.next_scope_index,
            flags,
        });
        self.next_scope_index += 1;
    }

    fn leave_scope(&mut self) {
        self.scopes.pop();
    }

    fn visit_function(&mut self, function: &Function<'a>, flags: ScopeFlags) {
        if matches!(
            function.r#type,
            FunctionType::FunctionDeclaration | FunctionType::TSDeclareFunction
        ) {
            if let (Some(id), Some(body)) = (&function.id, function.body.as_ref()) {
                if let Some(usage) =
                    extract_then_callback_function_usage(&function.params, body)
                {
                    self.register_in_nearest_var_scope(
                        id.name.as_str().to_string(),
                        usage,
                    );
                }
            }
        }

        walk::walk_function(self, function, flags);
    }
}

fn collect_require_dynamic_bindings(
    bindings: &mut DynamicBindings,
    pattern: &BindingPattern<'_>,
    source: &str,
) {
    match source {
        "react" => collect_react_require_bindings(bindings, pattern),
        "next/dynamic" => collect_next_dynamic_require_bindings(bindings, pattern),
        _ => {}
    }
}

fn collect_require_dynamic_binding_name(
    bindings: &mut DynamicBindings,
    source: &str,
    local_name: &str,
) {
    match source {
        "react" => {
            bindings.react_objects.insert(local_name.to_string());
        }
        "next/dynamic" => {
            bindings
                .next_dynamic_functions
                .insert(local_name.to_string());
        }
        _ => {}
    }
}

fn collect_react_require_bindings(bindings: &mut DynamicBindings, pattern: &BindingPattern<'_>) {
    match pattern {
        BindingPattern::BindingIdentifier(identifier) => {
            bindings
                .react_objects
                .insert(identifier.name.as_str().to_string());
        }
        BindingPattern::ObjectPattern(object_pattern) => {
            if object_pattern.rest.is_some() {
                return;
            }

            for property in &object_pattern.properties {
                if property.computed {
                    continue;
                }

                let Some(imported) = property_key_to_string(&property.key) else {
                    continue;
                };
                let Some(local) = binding_pattern_name(&property.value) else {
                    continue;
                };

                if imported == "default" {
                    bindings.react_objects.insert(local);
                } else if imported == "lazy" {
                    bindings.react_lazy_functions.insert(local);
                }
            }
        }
        BindingPattern::ArrayPattern(_) | BindingPattern::AssignmentPattern(_) => {}
    }
}

fn collect_next_dynamic_require_bindings(
    bindings: &mut DynamicBindings,
    pattern: &BindingPattern<'_>,
) {
    match pattern {
        BindingPattern::BindingIdentifier(identifier) => {
            bindings
                .next_dynamic_functions
                .insert(identifier.name.as_str().to_string());
        }
        BindingPattern::ObjectPattern(object_pattern) => {
            if object_pattern.rest.is_some() {
                return;
            }

            for property in &object_pattern.properties {
                if property.computed {
                    continue;
                }

                let Some(imported) = property_key_to_string(&property.key) else {
                    continue;
                };
                let Some(local) = binding_pattern_name(&property.value) else {
                    continue;
                };

                if imported == "default" {
                    bindings.next_dynamic_functions.insert(local);
                }
            }
        }
        BindingPattern::ArrayPattern(_) | BindingPattern::AssignmentPattern(_) => {}
    }
}

fn binding_pattern_name(pattern: &BindingPattern<'_>) -> Option<String> {
    match pattern {
        BindingPattern::BindingIdentifier(identifier) => Some(identifier.name.as_str().to_string()),
        BindingPattern::AssignmentPattern(pattern) => pattern
            .left
            .get_binding_identifier()
            .map(|identifier| identifier.name.as_str().to_string()),
        BindingPattern::ObjectPattern(_) | BindingPattern::ArrayPattern(_) => None,
    }
}

fn collect_shadowed_dynamic_bindings(program: &Program<'_>) -> BTreeMap<usize, BTreeSet<String>> {
    let mut collector = ShadowedDynamicBindingCollector::new();
    collector.visit_program(program);
    collector.shadowed
}

struct ShadowedDynamicBindingCollector {
    scopes: Vec<ScopeEntry>,
    shadowed: BTreeMap<usize, BTreeSet<String>>,
    next_scope_index: usize,
}

#[derive(Clone, Copy)]
struct ScopeEntry {
    index: usize,
    flags: ScopeFlags,
}

impl ShadowedDynamicBindingCollector {
    fn new() -> Self {
        Self {
            scopes: Vec::new(),
            shadowed: BTreeMap::new(),
            next_scope_index: 0,
        }
    }

    fn register_in_current_scope(&mut self, name: &str) {
        let Some(scope) = self.scopes.last() else {
            return;
        };

        self.shadowed
            .entry(scope.index)
            .or_default()
            .insert(name.to_string());
    }

    fn register_in_nearest_var_scope(&mut self, name: &str) {
        let Some(scope) = self.scopes.iter().rev().find(|scope| scope.flags.is_var()) else {
            return;
        };

        self.shadowed
            .entry(scope.index)
            .or_default()
            .insert(name.to_string());
    }
}

impl<'a> Visit<'a> for ShadowedDynamicBindingCollector {
    fn enter_scope(&mut self, flags: ScopeFlags, _scope_id: &Cell<Option<ScopeId>>) {
        self.scopes.push(ScopeEntry {
            index: self.next_scope_index,
            flags,
        });
        self.next_scope_index += 1;
    }

    fn leave_scope(&mut self) {
        self.scopes.pop();
    }

    fn visit_binding_identifier(&mut self, identifier: &BindingIdentifier<'a>) {
        self.register_in_current_scope(identifier.name.as_str());
    }

    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        if declarator.kind == VariableDeclarationKind::Var {
            for identifier in declarator.id.get_binding_identifiers() {
                self.register_in_nearest_var_scope(identifier.name.as_str());
            }

            if let Some(init) = &declarator.init {
                self.visit_expression(init);
            }
            return;
        }

        walk::walk_variable_declarator(self, declarator);
    }

    fn visit_function(&mut self, function: &Function<'a>, flags: ScopeFlags) {
        if matches!(
            function.r#type,
            FunctionType::FunctionDeclaration | FunctionType::TSDeclareFunction
        ) {
            if let Some(id) = &function.id {
                self.register_in_current_scope(id.name.as_str());
            }
        }

        walk::walk_function(self, function, flags);
    }
}

fn collect_react_import_bindings(
    bindings: &mut DynamicBindings,
    specifiers: Option<&[ImportDeclarationSpecifier<'_>]>,
) {
    let Some(specifiers) = specifiers else {
        return;
    };

    for specifier in specifiers {
        match specifier {
            ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier) => {
                bindings
                    .react_objects
                    .insert(specifier.local.name.as_str().to_string());
            }
            ImportDeclarationSpecifier::ImportNamespaceSpecifier(specifier) => {
                bindings
                    .react_objects
                    .insert(specifier.local.name.as_str().to_string());
            }
            ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                let imported = module_export_name_to_string(&specifier.imported)
                    .unwrap_or_else(|| specifier.local.name.as_str().to_string());
                let local = specifier.local.name.as_str().to_string();

                if imported == "default" {
                    bindings.react_objects.insert(local);
                } else if imported == "lazy" {
                    bindings.react_lazy_functions.insert(local);
                }
            }
        }
    }
}

fn collect_next_dynamic_import_bindings(
    bindings: &mut DynamicBindings,
    specifiers: Option<&[ImportDeclarationSpecifier<'_>]>,
) {
    let Some(specifiers) = specifiers else {
        return;
    };

    for specifier in specifiers {
        let local = match specifier {
            ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier) => {
                Some(specifier.local.name.as_str())
            }
            ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => None,
            ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                let imported = module_export_name_to_string(&specifier.imported)
                    .unwrap_or_else(|| specifier.local.name.as_str().to_string());
                (imported == "default").then_some(specifier.local.name.as_str())
            }
        };

        if let Some(local) = local {
            bindings.next_dynamic_functions.insert(local.to_string());
        }
    }
}

fn extract_dynamic_wrapper_usage(
    call: &CallExpression<'_>,
    bindings: &DynamicBindings,
    active_bindings: &DynamicBindingCounts,
    active_loader_bindings: &BTreeMap<String, Vec<TrackedDynamicImport>>,
    shadowed: &BTreeMap<String, usize>,
) -> Option<TrackedDynamicImport> {
    let callee = call.callee.without_parentheses();
    let callback = call
        .arguments
        .first()?
        .as_expression()?
        .without_parentheses();

    if is_react_lazy_callee(callee, bindings, active_bindings, shadowed) {
        return extract_loader_callback_usage(callback, active_loader_bindings, shadowed);
    }

    if is_next_dynamic_callee(callee, bindings, active_bindings, shadowed) {
        return extract_loader_callback_usage(callback, active_loader_bindings, shadowed);
    }

    None
}

fn extract_loader_callback_usage(
    callback: &Expression<'_>,
    active_loader_bindings: &BTreeMap<String, Vec<TrackedDynamicImport>>,
    shadowed: &BTreeMap<String, usize>,
) -> Option<TrackedDynamicImport> {
    match callback {
        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_) => {
            extract_loader_expression_usage(callback)
        }
        Expression::Identifier(identifier) => {
            let name = identifier.name.as_str();
            let shadow_count = shadowed.get(name).copied().unwrap_or_default();
            let active_bindings = active_loader_bindings.get(name)?;

            if shadow_count > active_bindings.len() {
                return None;
            }

            active_bindings.last().cloned()
        }
        _ => None,
    }
}

fn extract_loader_binding_usage(expression: &Expression<'_>) -> Option<TrackedDynamicImport> {
    match expression.without_parentheses() {
        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_) => {
            extract_loader_expression_usage(expression)
        }
        _ => None,
    }
}

fn classify_dynamic_alias_expression(
    expression: &Expression<'_>,
    bindings: &DynamicBindings,
    active_bindings: &DynamicBindingCounts,
    shadowed: &BTreeMap<String, usize>,
) -> Option<DynamicBindingKind> {
    let expression = expression.without_parentheses();

    match expression {
        Expression::Identifier(identifier) => {
            let name = identifier.name.as_str();
            if binding_name_matches(
                name,
                &bindings.react_objects,
                &active_bindings.react_objects,
            ) && !is_shadowed(name, shadowed, &active_bindings.react_objects)
            {
                return Some(DynamicBindingKind::ReactObject);
            }

            if binding_name_matches(
                name,
                &bindings.react_lazy_functions,
                &active_bindings.react_lazy_functions,
            ) && !is_shadowed(name, shadowed, &active_bindings.react_lazy_functions)
            {
                return Some(DynamicBindingKind::ReactLazyFunction);
            }

            if binding_name_matches(
                name,
                &bindings.next_dynamic_functions,
                &active_bindings.next_dynamic_functions,
            ) && !is_shadowed(name, shadowed, &active_bindings.next_dynamic_functions)
            {
                return Some(DynamicBindingKind::NextDynamicFunction);
            }

            None
        }
        Expression::StaticMemberExpression(member) => {
            if member.property.name.as_str() == "lazy"
                && is_react_wrapper_object(&member.object, bindings, active_bindings, shadowed)
            {
                return Some(DynamicBindingKind::ReactLazyFunction);
            }

            if member.property.name.as_str() == "default"
                && matches!(
                    &member.object.without_parentheses(),
                    Expression::CallExpression(call)
                        if require_call_matches_source(call, "next/dynamic", shadowed)
                )
            {
                return Some(DynamicBindingKind::NextDynamicFunction);
            }

            None
        }
        _ => None,
    }
}

fn is_react_lazy_callee(
    expression: &Expression<'_>,
    bindings: &DynamicBindings,
    active_bindings: &DynamicBindingCounts,
    shadowed: &BTreeMap<String, usize>,
) -> bool {
    match expression {
        Expression::StaticMemberExpression(member) => {
            member.property.name.as_str() == "lazy"
                && is_react_wrapper_object(
                    &member.object,
                    bindings,
                    active_bindings,
                    shadowed,
                )
        }
        Expression::Identifier(identifier) => {
            binding_name_matches(
                identifier.name.as_str(),
                &bindings.react_lazy_functions,
                &active_bindings.react_lazy_functions,
            )
                && !is_shadowed(
                    identifier.name.as_str(),
                    shadowed,
                    &active_bindings.react_lazy_functions,
                )
        }
        _ => false,
    }
}

fn is_next_dynamic_callee(
    expression: &Expression<'_>,
    bindings: &DynamicBindings,
    active_bindings: &DynamicBindingCounts,
    shadowed: &BTreeMap<String, usize>,
) -> bool {
    match expression {
        Expression::Identifier(identifier) => {
            binding_name_matches(
                identifier.name.as_str(),
                &bindings.next_dynamic_functions,
                &active_bindings.next_dynamic_functions,
            ) && !is_shadowed(
                identifier.name.as_str(),
                shadowed,
                &active_bindings.next_dynamic_functions,
            )
        }
        Expression::StaticMemberExpression(member) => {
            member.property.name.as_str() == "default"
                && matches!(
                    &member.object.without_parentheses(),
                    Expression::CallExpression(call)
                        if require_call_matches_source(call, "next/dynamic", shadowed)
                )
        }
        _ => false,
    }
}

fn is_react_wrapper_object(
    expression: &Expression<'_>,
    bindings: &DynamicBindings,
    active_bindings: &DynamicBindingCounts,
    shadowed: &BTreeMap<String, usize>,
) -> bool {
    match expression.without_parentheses() {
        Expression::Identifier(identifier) => {
            binding_name_matches(
                identifier.name.as_str(),
                &bindings.react_objects,
                &active_bindings.react_objects,
            ) && !is_shadowed(
                identifier.name.as_str(),
                shadowed,
                &active_bindings.react_objects,
            )
        }
        Expression::CallExpression(call) => require_call_matches_source(call, "react", shadowed),
        Expression::StaticMemberExpression(member) => {
            member.property.name.as_str() == "default"
                && matches!(
                    &member.object.without_parentheses(),
                    Expression::CallExpression(call)
                        if require_call_matches_source(call, "react", shadowed)
                )
        }
        _ => false,
    }
}

fn require_call_matches_source(
    call: &CallExpression<'_>,
    source: &str,
    shadowed: &BTreeMap<String, usize>,
) -> bool {
    if shadowed.get("require").copied().unwrap_or_default() > 0 {
        return false;
    }

    if !call.callee.without_parentheses().is_specific_id("require") {
        return false;
    }

    if call.arguments.len() != 1 {
        return false;
    }

    let Some(argument) = call.arguments.first() else {
        return false;
    };
    let Some(expression) = argument.as_expression() else {
        return false;
    };
    let Expression::StringLiteral(literal) = expression.without_parentheses() else {
        return false;
    };

    literal.value.as_str() == source
}

fn is_shadowed(
    name: &str,
    shadowed: &BTreeMap<String, usize>,
    active_bindings: &BTreeMap<String, usize>,
) -> bool {
    let shadow_count = shadowed.get(name).copied().unwrap_or_default();
    let active_count = active_bindings.get(name).copied().unwrap_or_default();
    shadow_count > active_count
}

fn binding_name_matches(
    name: &str,
    global_bindings: &BTreeSet<String>,
    active_bindings: &BTreeMap<String, usize>,
) -> bool {
    global_bindings.contains(name) || active_bindings.get(name).copied().unwrap_or_default() > 0
}

fn extract_loader_expression_usage(expression: &Expression<'_>) -> Option<TrackedDynamicImport> {
    let expression = expression.without_parentheses();

    match expression.without_parentheses() {
        Expression::ImportExpression(import) => {
            let Expression::StringLiteral(literal) = &import.source else {
                return None;
            };

            Some(TrackedDynamicImport {
                source: literal.value.as_str().to_string(),
                usage: DynamicImportUsage::Default,
            })
        }
        Expression::ArrowFunctionExpression(arrow) => {
            let result = arrow
                .get_expression()
                .or_else(|| extract_function_body_return_expression(&arrow.body))?;
            extract_loader_expression_usage(result)
        }
        Expression::FunctionExpression(function) => {
            let body = function.body.as_ref()?;
            let result = extract_function_body_return_expression(body)?;
            extract_loader_expression_usage(result)
        }
        Expression::CallExpression(call) => extract_then_call_usage(call),
        _ => None,
    }
}

fn extract_then_call_usage(call: &CallExpression<'_>) -> Option<TrackedDynamicImport> {
    if call.arguments.is_empty() {
        return None;
    }

    let Expression::StaticMemberExpression(member) = call.callee.without_parentheses() else {
        return None;
    };

    if member.property.name.as_str() != "then" {
        return None;
    }

    let Expression::ImportExpression(import) = member.object.without_parentheses() else {
        return None;
    };
    let Expression::StringLiteral(literal) = &import.source else {
        return None;
    };

    let callback = call.arguments.first()?.as_expression()?;
    Some(TrackedDynamicImport {
        source: literal.value.as_str().to_string(),
        usage: extract_then_callback_usage(callback)?,
    })
}

fn extract_then_callback_usage(expression: &Expression<'_>) -> Option<DynamicImportUsage> {
    let expression = expression.without_parentheses();

    match expression {
        Expression::ArrowFunctionExpression(arrow) => {
            let binding = extract_then_callback_binding(&arrow.params)?;
            match arrow.get_expression() {
                Some(expression) => {
                    extract_selected_export_usage(expression, &binding, &BTreeMap::new())
                }
                None => extract_block_then_callback_usage(&arrow.body, &binding),
            }
        }
        Expression::FunctionExpression(function) => {
            extract_then_callback_function_usage(&function.params, function.body.as_ref()?)
        }
        _ => None,
    }
}

fn extract_then_callback_function_usage(
    params: &oxc_ast::ast::FormalParameters<'_>,
    body: &FunctionBody<'_>,
) -> Option<DynamicImportUsage> {
    let binding = extract_then_callback_binding(params)?;
    extract_block_then_callback_usage(body, &binding)
}

fn extract_block_then_callback_usage(
    body: &FunctionBody<'_>,
    binding: &ThenCallbackBinding,
) -> Option<DynamicImportUsage> {
    extract_then_callback_statements_usage(&body.statements, binding, &BTreeMap::new())
}

fn extract_then_callback_statements_usage(
    statements: &[Statement<'_>],
    binding: &ThenCallbackBinding,
    inherited_aliases: &BTreeMap<String, DynamicImportUsage>,
) -> Option<DynamicImportUsage> {
    let mut aliases = inherited_aliases.clone();

    for statement in statements {
        match statement {
            Statement::VariableDeclaration(variable) => {
                for declarator in &variable.declarations {
                    if let Some(alias_entries) =
                        extract_then_callback_alias_entries(declarator, binding, &aliases)
                    {
                        for (local_name, usage) in alias_entries {
                            aliases.insert(local_name, usage);
                        }
                    }
                }
            }
            Statement::FunctionDeclaration(function) => {
                let Some(id) = &function.id else {
                    return None;
                };
                if let Some(usage) =
                    extract_then_callback_local_function_usage(function, binding, &aliases)
                {
                    aliases.insert(id.name.as_str().to_string(), usage);
                }
            }
            Statement::ReturnStatement(statement) => {
                let expression = statement.argument.as_ref()?;
                return extract_selected_export_usage(expression, binding, &aliases);
            }
            Statement::BlockStatement(block) => {
                if let Some(usage) =
                    extract_then_callback_statements_usage(&block.body, binding, &aliases)
                {
                    return Some(usage);
                }
            }
            _ => return None,
        }
    }

    None
}

fn extract_then_callback_alias_entries(
    declarator: &VariableDeclarator<'_>,
    binding: &ThenCallbackBinding,
    aliases: &BTreeMap<String, DynamicImportUsage>,
) -> Option<Vec<(String, DynamicImportUsage)>> {
    if let Some(local_name) = binding_pattern_name(&declarator.id) {
        let initializer = declarator.init.as_ref()?;
        let usage = extract_selected_export_usage(initializer, binding, aliases)?;
        return Some(vec![(local_name, usage)]);
    }

    let initializer = declarator.init.as_ref()?;
    let Expression::Identifier(identifier) = initializer.without_parentheses() else {
        return None;
    };
    let ThenCallbackBinding::Namespace(param_name) = binding else {
        return None;
    };
    if identifier.name.as_str() != param_name {
        return None;
    }

    let BindingPattern::ObjectPattern(pattern) = &declarator.id else {
        return None;
    };
    if pattern.rest.is_some() {
        return None;
    }

    let mut entries = Vec::new();
    for property in &pattern.properties {
        if property.computed {
            return None;
        }

        let imported = property_key_to_string(&property.key)?;
        let local = match &property.value {
            BindingPattern::BindingIdentifier(identifier) => {
                identifier.name.as_str().to_string()
            }
            BindingPattern::AssignmentPattern(pattern) => pattern
                .left
                .get_binding_identifier()
                .map(|identifier| identifier.name.as_str().to_string())?,
            BindingPattern::ObjectPattern(_) | BindingPattern::ArrayPattern(_) => {
                return None;
            }
        };

        entries.push((local, dynamic_usage_for_export(&imported)));
    }

    Some(entries)
}

fn extract_then_callback_local_function_usage(
    function: &Function<'_>,
    binding: &ThenCallbackBinding,
    aliases: &BTreeMap<String, DynamicImportUsage>,
) -> Option<DynamicImportUsage> {
    if !function.params.items.is_empty() || function.params.rest.is_some() {
        return None;
    }

    let body = function.body.as_ref()?;
    let expression = extract_function_body_return_expression(body)?;
    extract_selected_export_usage(expression, binding, aliases)
}

fn extract_then_callback_binding(
    params: &oxc_ast::ast::FormalParameters<'_>,
) -> Option<ThenCallbackBinding> {
    if params.items.len() != 1 || params.rest.is_some() {
        return None;
    }

    match &params.items[0].pattern {
        BindingPattern::BindingIdentifier(identifier) => {
            Some(ThenCallbackBinding::Namespace(
                identifier.name.as_str().to_string(),
            ))
        }
        BindingPattern::ObjectPattern(pattern) => {
            if pattern.rest.is_some() {
                return None;
            }

            let mut bindings = BTreeMap::new();

            for property in &pattern.properties {
                if property.computed {
                    return None;
                }

                let imported = property_key_to_string(&property.key)?;
                let local = match &property.value {
                    BindingPattern::BindingIdentifier(identifier) => {
                        identifier.name.as_str().to_string()
                    }
                    BindingPattern::AssignmentPattern(pattern) => pattern
                        .left
                        .get_binding_identifier()
                        .map(|identifier| identifier.name.as_str().to_string())?,
                    BindingPattern::ObjectPattern(_) | BindingPattern::ArrayPattern(_) => {
                        return None;
                    }
                };

                bindings.insert(local, imported);
            }

            Some(ThenCallbackBinding::Destructured(bindings))
        }
        BindingPattern::ArrayPattern(_) | BindingPattern::AssignmentPattern(_) => None,
    }
}

fn extract_function_body_return_expression<'a>(
    body: &'a FunctionBody<'a>,
) -> Option<&'a Expression<'a>> {
    if body.statements.len() != 1 {
        return None;
    }

    let Statement::ReturnStatement(statement) = &body.statements[0] else {
        return None;
    };

    statement.argument.as_ref()
}

fn extract_selected_export_usage(
    expression: &Expression<'_>,
    binding: &ThenCallbackBinding,
    aliases: &BTreeMap<String, DynamicImportUsage>,
) -> Option<DynamicImportUsage> {
    let expression = expression.without_parentheses();

    match expression {
        Expression::StaticMemberExpression(member) => {
            let ThenCallbackBinding::Namespace(param_name) = binding else {
                return None;
            };

            if !member.object.is_specific_id(param_name) {
                return None;
            }

            let export_name = member.property.name.as_str();
            if export_name == "default" {
                Some(DynamicImportUsage::Default)
            } else {
                Some(DynamicImportUsage::Named(export_name.to_string()))
            }
        }
        Expression::Identifier(identifier) => {
            if let Some(usage) = aliases.get(identifier.name.as_str()) {
                return Some(usage.clone());
            }

            match binding {
                ThenCallbackBinding::Destructured(bindings) => {
                    binding_usage_from_identifier(identifier.name.as_str(), bindings)
                }
                ThenCallbackBinding::Namespace(_) => None,
            }
        }
        Expression::CallExpression(call) => {
            if !call.arguments.is_empty() {
                return None;
            }

            let Expression::Identifier(identifier) = call.callee.without_parentheses() else {
                return None;
            };

            aliases.get(identifier.name.as_str()).cloned()
        }
        Expression::ObjectExpression(object) => {
            extract_named_export_from_object(object, binding, aliases)
        }
        _ => None,
    }
}

fn extract_named_export_from_object(
    object: &ObjectExpression<'_>,
    binding: &ThenCallbackBinding,
    aliases: &BTreeMap<String, DynamicImportUsage>,
) -> Option<DynamicImportUsage> {
    if object.properties.len() != 1 {
        return None;
    }

    let ObjectPropertyKind::ObjectProperty(property) = &object.properties[0] else {
        return None;
    };

    if property.kind != PropertyKind::Init
        || property.computed
        || property.method
        || property.shorthand
    {
        return None;
    }

    let Some(key) = property_key_to_string(&property.key) else {
        return None;
    };

    if key != "default" {
        return None;
    }

    let value = property.value.without_parentheses();
    match value {
        Expression::StaticMemberExpression(member) => {
            let ThenCallbackBinding::Namespace(param_name) = binding else {
                return None;
            };

            if !member.object.is_specific_id(param_name) {
                return None;
            }

            let export_name = member.property.name.as_str();
            if export_name == "default" {
                Some(DynamicImportUsage::Default)
            } else {
                Some(DynamicImportUsage::Named(export_name.to_string()))
            }
        }
        Expression::Identifier(identifier) => {
            if let Some(usage) = aliases.get(identifier.name.as_str()) {
                return Some(usage.clone());
            }

            let ThenCallbackBinding::Destructured(bindings) = binding else {
                return None;
            };

            binding_usage_from_identifier(identifier.name.as_str(), bindings)
        }
        Expression::CallExpression(call) => {
            if !call.arguments.is_empty() {
                return None;
            }

            let Expression::Identifier(identifier) = call.callee.without_parentheses() else {
                return None;
            };

            aliases.get(identifier.name.as_str()).cloned()
        }
        _ => None,
    }
}

fn dynamic_usage_for_export(export_name: &str) -> DynamicImportUsage {
    if export_name == "default" {
        DynamicImportUsage::Default
    } else {
        DynamicImportUsage::Named(export_name.to_string())
    }
}

fn binding_usage_from_identifier(
    identifier: &str,
    bindings: &BTreeMap<String, String>,
) -> Option<DynamicImportUsage> {
    let imported = bindings.get(identifier)?;
    if imported == "default" {
        Some(DynamicImportUsage::Default)
    } else {
        Some(DynamicImportUsage::Named(imported.clone()))
    }
}

fn dynamic_usage_specifiers(usage: &DynamicImportUsage) -> Vec<ImportSpecifier> {
    match usage {
        DynamicImportUsage::Default => vec![ImportSpecifier {
            kind: ImportSpecifierKind::Default,
            imported: Some("default".to_string()),
            local: None,
        }],
        DynamicImportUsage::Named(name) => vec![ImportSpecifier {
            kind: ImportSpecifierKind::Named,
            imported: Some(name.clone()),
            local: None,
        }],
    }
}

fn extract_require_source(
    expression: Option<&Expression<'_>>,
    require_shadowed: bool,
) -> Option<String> {
    if require_shadowed {
        return None;
    }

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
        Expression::StaticMemberExpression(member) => {
            extract_require_source(Some(&member.object), require_shadowed)
        }
        Expression::ComputedMemberExpression(member) => {
            extract_require_source(Some(&member.object), require_shadowed)
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
