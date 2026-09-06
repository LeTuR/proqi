//! Executable single-source ownership policy for semantic keyboard bindings.

use std::{collections::BTreeSet, path::Path};

mod cfg_policy;

use cfg_policy::has_test_only_configuration;

const REGISTRY_ROOT: &str = "src/ui/shortcut_registry/";
const TERMINAL_TRANSLATION: &str = "src/adapters/terminal/input/translation.rs";
const KEYSTROKE_MODEL: &str = "src/ui/input/keystroke.rs";
const UI_REEXPORTS: &[&str] = &["src/ui/input.rs", "src/ui/mod.rs"];
const KEYBINDING_PROJECTION_OWNERS: &[&str] = &[
    "src/ui/settings.rs",
    "src/adapters/terminal/settings.rs",
    "src/ui/app.rs",
    "src/ui/app/view.rs",
    "src/ui/app/view_frame.rs",
];

pub(crate) fn required_owner_findings(root: &Path) -> Vec<String> {
    [
        "src/ui/input/keystroke.rs",
        "src/ui/shortcut_registry/model.rs",
        "src/ui/shortcut_registry/inventory.rs",
        "src/ui/shortcut_registry/dispatch.rs",
        TERMINAL_TRANSLATION,
    ]
    .into_iter()
    .filter(|relative| !root.join(relative).is_file())
    .map(|relative| format!("{relative}: required shortcut architecture owner is missing"))
    .collect()
}

pub(crate) fn check_source(path: &Path, source: &str) -> Vec<String> {
    let path_text = slash_path(path);
    if is_test_fixture(&path_text) {
        return Vec::new();
    }
    let file = match syn::parse_file(source) {
        Ok(file) => file,
        Err(error) => {
            return vec![format!(
                "{}: shortcut architecture scan could not parse Rust source: {error}",
                path.display()
            )];
        }
    };
    let mut visitor = ShortcutVisitor::default();
    syn::visit::Visit::visit_file(&mut visitor, &file);
    ownership_findings(path, &path_text, &visitor)
}

fn ownership_findings(path: &Path, path_text: &str, visitor: &ShortcutVisitor) -> Vec<String> {
    let mut findings = Vec::new();

    if visitor.detected.contains(&Detected::CrosstermKeyTypes) && path_text != TERMINAL_TRANSLATION
    {
        findings.push(format!(
            "{}: Crossterm key interpretation is outside the terminal translation boundary",
            path.display()
        ));
    }
    let owns_logical_keys = path_text.starts_with(REGISTRY_ROOT)
        || path_text == TERMINAL_TRANSLATION
        || path_text == KEYSTROKE_MODEL
        || UI_REEXPORTS.contains(&path_text);
    if visitor.detected.contains(&Detected::LogicalKey) && !owns_logical_keys {
        findings.push(format!(
            "{}: raw LogicalKey interpretation is outside the shortcut registry dispatcher",
            path.display()
        ));
    }
    let owns_bindings = path_text.starts_with(REGISTRY_ROOT) || path_text == "src/ui/mod.rs";
    if visitor.detected.contains(&Detected::ShortcutBinding) && !owns_bindings {
        findings.push(format!(
            "{}: ShortcutBinding declarations are outside the shortcut registry",
            path.display()
        ));
    }
    if path_text == "src/ui/shortcut_metadata.rs" {
        findings.push(format!(
            "{}: parallel shortcut metadata must be projected from the registry",
            path.display()
        ));
    }
    if visitor.detected.contains(&Detected::CommandsInventory)
        && path_text != "src/ui/shortcut_registry/model.rs"
    {
        findings.push(format!(
            "{}: Commands inventory is outside the shortcut registry action owner",
            path.display()
        ));
    }
    if visitor
        .detected
        .contains(&Detected::SemanticCharacterLiteral)
        && !path_text.starts_with(REGISTRY_ROOT)
    {
        findings.push(format!(
            "{}: semantic character binding is outside the shortcut registry",
            path.display()
        ));
    }
    if visitor
        .detected
        .contains(&Detected::SemanticKeyPresentation)
        && !path_text.starts_with(REGISTRY_ROOT)
    {
        findings.push(format!(
            "{}: semantic shortcut presentation is outside the shortcut registry",
            path.display()
        ));
    }
    if visitor
        .detected
        .contains(&Detected::StandaloneShortcutKeyConstant)
        && !path_text.starts_with(REGISTRY_ROOT)
    {
        findings.push(format!(
            "{}: standalone shortcut key constant is outside the shortcut registry",
            path.display()
        ));
    }
    if visitor.detected.contains(&Detected::KeybindingsAccess)
        && !path_text.starts_with(REGISTRY_ROOT)
        && !KEYBINDING_PROJECTION_OWNERS.contains(&path_text)
    {
        findings.push(format!(
            "{}: configured keybinding access is outside registry loading or presentation",
            path.display()
        ));
    }
    findings
}

#[derive(Default)]
struct ShortcutVisitor {
    detected: BTreeSet<Detected>,
    semantic_character_bindings: Vec<Vec<String>>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Detected {
    CrosstermKeyTypes,
    LogicalKey,
    ShortcutBinding,
    CommandsInventory,
    SemanticCharacterLiteral,
    SemanticKeyPresentation,
    StandaloneShortcutKeyConstant,
    KeybindingsAccess,
}

impl<'ast> syn::visit::Visit<'ast> for ShortcutVisitor {
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if !has_test_only_configuration(&item.attrs) {
            syn::visit::visit_item_mod(self, item);
        }
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if !has_test_only_configuration(&item.attrs) {
            syn::visit::visit_item_fn(self, item);
        }
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        if !has_test_only_configuration(&item.attrs) {
            syn::visit::visit_impl_item_fn(self, item);
        }
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        if !has_test_only_configuration(&item.attrs) {
            syn::visit::visit_item_impl(self, item);
        }
    }

    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        if !has_test_only_configuration(&item.attrs) {
            if item.ident.to_string().ends_with("_KEY") && is_char_type(&item.ty) {
                self.detected
                    .insert(Detected::StandaloneShortcutKeyConstant);
            }
            syn::visit::visit_item_const(self, item);
        }
    }

    fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
        if !has_test_only_configuration(&item.attrs) {
            if item.ident.to_string().ends_with("_KEY") && is_char_type(&item.ty) {
                self.detected
                    .insert(Detected::StandaloneShortcutKeyConstant);
            }
            syn::visit::visit_item_static(self, item);
        }
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        for segment in &path.segments {
            match segment.ident.to_string().as_str() {
                "KeyCode" | "KeyEvent" | "KeyEventKind" | "KeyEventState" | "KeyModifiers" => {
                    self.detected.insert(Detected::CrosstermKeyTypes);
                }
                "LogicalKey" => {
                    self.detected.insert(Detected::LogicalKey);
                }
                "ShortcutBinding" => {
                    self.detected.insert(Detected::ShortcutBinding);
                }
                _ => {}
            }
        }
        syn::visit::visit_path(self, path);
    }

    fn visit_ident(&mut self, ident: &'ast syn::Ident) {
        match ident.to_string().as_str() {
            "KeyCode" | "KeyEvent" | "KeyEventKind" | "KeyEventState" | "KeyModifiers" => {
                self.detected.insert(Detected::CrosstermKeyTypes);
            }
            "LogicalKey" => {
                self.detected.insert(Detected::LogicalKey);
            }
            "ShortcutBinding" => {
                self.detected.insert(Detected::ShortcutBinding);
            }
            _ => {}
        }
    }

    fn visit_impl_item_const(&mut self, item: &'ast syn::ImplItemConst) {
        if item.ident == "COMMANDS" {
            self.detected.insert(Detected::CommandsInventory);
        }
        syn::visit::visit_impl_item_const(self, item);
    }

    fn visit_pat_tuple_struct(&mut self, pattern: &'ast syn::PatTupleStruct) {
        let semantic_character = pattern.path.segments.last().is_some_and(|segment| {
            matches!(
                segment.ident.to_string().as_str(),
                "Character" | "PrimaryCharacter" | "PrimaryShiftCharacter"
            )
        });
        if semantic_character
            && pattern
                .elems
                .iter()
                .any(|element| matches!(element, syn::Pat::Lit(_)))
        {
            self.detected.insert(Detected::SemanticCharacterLiteral);
        }
        syn::visit::visit_pat_tuple_struct(self, pattern);
    }

    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        let mut bindings = Vec::new();
        collect_semantic_character_bindings(&arm.pat, &mut bindings);
        self.semantic_character_bindings.push(bindings);
        syn::visit::visit_pat(self, &arm.pat);
        syn::visit::visit_expr(self, &arm.body);
        self.semantic_character_bindings.pop();
    }

    fn visit_expr_match(&mut self, expression: &'ast syn::ExprMatch) {
        let matched_binding = match expression.expr.as_ref() {
            syn::Expr::Path(path) => path.path.get_ident().map(ToString::to_string),
            _ => None,
        };
        if matched_binding.is_some_and(|binding| {
            self.semantic_character_bindings
                .iter()
                .flatten()
                .any(|candidate| candidate == &binding)
        }) && expression
            .arms
            .iter()
            .any(|arm| pattern_contains_character_literal(&arm.pat))
        {
            self.detected.insert(Detected::SemanticCharacterLiteral);
        }
        syn::visit::visit_expr_match(self, expression);
    }

    fn visit_expr_if(&mut self, expression: &'ast syn::ExprIf) {
        let mut bindings = Vec::new();
        collect_condition_semantic_bindings(&expression.cond, &mut bindings);
        self.semantic_character_bindings.push(bindings);
        syn::visit::visit_expr_if(self, expression);
        self.semantic_character_bindings.pop();
    }

    fn visit_expr_while(&mut self, expression: &'ast syn::ExprWhile) {
        let mut bindings = Vec::new();
        collect_condition_semantic_bindings(&expression.cond, &mut bindings);
        self.semantic_character_bindings.push(bindings);
        syn::visit::visit_expr_while(self, expression);
        self.semantic_character_bindings.pop();
    }

    fn visit_expr_binary(&mut self, expression: &'ast syn::ExprBinary) {
        if matches!(expression.op, syn::BinOp::Eq(_) | syn::BinOp::Ne(_))
            && ((self.is_semantic_character_binding(&expression.left)
                && is_character_literal(&expression.right))
                || (self.is_semantic_character_binding(&expression.right)
                    && is_character_literal(&expression.left)))
        {
            self.detected.insert(Detected::SemanticCharacterLiteral);
        }
        syn::visit::visit_expr_binary(self, expression);
    }

    fn visit_expr_macro(&mut self, expression: &'ast syn::ExprMacro) {
        if expression.mac.path.is_ident("matches")
            && token_stream_contains_character_literal(&expression.mac.tokens)
            && (token_stream_contains_semantic_variant(&expression.mac.tokens)
                || self
                    .semantic_character_bindings
                    .iter()
                    .flatten()
                    .any(|binding| token_stream_contains_ident(&expression.mac.tokens, binding)))
        {
            self.detected.insert(Detected::SemanticCharacterLiteral);
        }
        syn::visit::visit_expr_macro(self, expression);
    }

    fn visit_expr_array(&mut self, expression: &'ast syn::ExprArray) {
        if expression
            .elems
            .iter()
            .any(is_semantic_key_presentation_tuple)
        {
            self.detected.insert(Detected::SemanticKeyPresentation);
        }
        syn::visit::visit_expr_array(self, expression);
    }

    fn visit_expr_field(&mut self, field: &'ast syn::ExprField) {
        if matches!(&field.member, syn::Member::Named(identifier) if identifier == "keybindings") {
            self.detected.insert(Detected::KeybindingsAccess);
        }
        syn::visit::visit_expr_field(self, field);
    }
}

impl ShortcutVisitor {
    fn is_semantic_character_binding(&self, expression: &syn::Expr) -> bool {
        let syn::Expr::Path(path) = expression else {
            return false;
        };
        let Some(identifier) = path.path.get_ident() else {
            return false;
        };
        self.semantic_character_bindings
            .iter()
            .flatten()
            .any(|binding| identifier == binding)
    }
}

fn collect_condition_semantic_bindings(expression: &syn::Expr, bindings: &mut Vec<String>) {
    match expression {
        syn::Expr::Let(let_expression) => {
            collect_semantic_character_bindings(&let_expression.pat, bindings);
        }
        syn::Expr::Binary(binary) => {
            collect_condition_semantic_bindings(&binary.left, bindings);
            collect_condition_semantic_bindings(&binary.right, bindings);
        }
        syn::Expr::Group(group) => collect_condition_semantic_bindings(&group.expr, bindings),
        syn::Expr::Paren(paren) => collect_condition_semantic_bindings(&paren.expr, bindings),
        _ => {}
    }
}

fn is_character_literal(expression: &syn::Expr) -> bool {
    matches!(expression, syn::Expr::Lit(literal) if matches!(literal.lit, syn::Lit::Char(_)))
}

fn is_char_type(value: &syn::Type) -> bool {
    matches!(value, syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident("char"))
}

fn token_stream_contains_character_literal(tokens: &proc_macro2::TokenStream) -> bool {
    tokens.clone().into_iter().any(|token| match token {
        proc_macro2::TokenTree::Literal(literal) => {
            matches!(
                syn::parse_str::<syn::Lit>(&literal.to_string()),
                Ok(syn::Lit::Char(_))
            )
        }
        proc_macro2::TokenTree::Group(group) => {
            token_stream_contains_character_literal(&group.stream())
        }
        _ => false,
    })
}

fn token_stream_contains_semantic_variant(tokens: &proc_macro2::TokenStream) -> bool {
    ["Character", "PrimaryCharacter", "PrimaryShiftCharacter"]
        .iter()
        .any(|identifier| token_stream_contains_ident(tokens, identifier))
}

fn token_stream_contains_ident(tokens: &proc_macro2::TokenStream, expected: &str) -> bool {
    tokens.clone().into_iter().any(|token| match token {
        proc_macro2::TokenTree::Ident(identifier) => identifier == expected,
        proc_macro2::TokenTree::Group(group) => {
            token_stream_contains_ident(&group.stream(), expected)
        }
        _ => false,
    })
}

fn is_semantic_key_presentation_tuple(expression: &syn::Expr) -> bool {
    let syn::Expr::Tuple(tuple) = expression else {
        return false;
    };
    let mut elements = tuple.elems.iter();
    let Some(syn::Expr::Path(owner)) = elements.next() else {
        return false;
    };
    if !owner.path.segments.iter().any(|segment| {
        matches!(
            segment.ident.to_string().as_str(),
            "BrowserHit" | "HitTarget"
        )
    }) {
        return false;
    }
    let Some(syn::Expr::Lit(key)) = elements.next() else {
        return false;
    };
    matches!(&key.lit, syn::Lit::Str(value) if value.value().chars().count() <= 8)
}

fn collect_semantic_character_bindings(pattern: &syn::Pat, bindings: &mut Vec<String>) {
    match pattern {
        syn::Pat::TupleStruct(tuple)
            if tuple.path.segments.last().is_some_and(|segment| {
                matches!(
                    segment.ident.to_string().as_str(),
                    "Character" | "PrimaryCharacter" | "PrimaryShiftCharacter"
                )
            }) =>
        {
            for element in &tuple.elems {
                if let syn::Pat::Ident(identifier) = element {
                    bindings.push(identifier.ident.to_string());
                }
            }
        }
        syn::Pat::Or(or) => {
            for case in &or.cases {
                collect_semantic_character_bindings(case, bindings);
            }
        }
        syn::Pat::Guard(guard) => collect_semantic_character_bindings(&guard.pat, bindings),
        syn::Pat::Paren(paren) => collect_semantic_character_bindings(&paren.pat, bindings),
        syn::Pat::Reference(reference) => {
            collect_semantic_character_bindings(&reference.pat, bindings);
        }
        _ => {}
    }
}

fn pattern_contains_character_literal(pattern: &syn::Pat) -> bool {
    match pattern {
        syn::Pat::Lit(literal) => matches!(literal.lit, syn::Lit::Char(_)),
        syn::Pat::Or(or) => or.cases.iter().any(pattern_contains_character_literal),
        syn::Pat::Guard(guard) => pattern_contains_character_literal(&guard.pat),
        syn::Pat::Paren(paren) => pattern_contains_character_literal(&paren.pat),
        syn::Pat::Reference(reference) => pattern_contains_character_literal(&reference.pat),
        _ => false,
    }
}

fn is_test_fixture(path: &str) -> bool {
    path.ends_with("/tests.rs")
        || path.contains("/tests/")
        || path.ends_with("_tests.rs")
        || path.starts_with("tests/")
}

fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
#[path = "shortcut_architecture/tests.rs"]
mod tests;
