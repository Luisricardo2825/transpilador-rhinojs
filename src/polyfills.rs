use swc_common::{sync::Lrc, FileName, SourceMap};
use swc_ecma_ast::{AssignTarget, Callee, Expr, MemberProp, SimpleAssignTarget};
use swc_ecma_parser::{lexer::Lexer, EsSyntax, Parser, StringInput, Syntax};
use swc_ecma_visit::{Visit, VisitWith};

#[derive(Default)]
struct Features(String);

impl Features {
    fn add(&mut self, feature: &str) {
        if !self.0.contains(feature) {
            self.0.push_str(feature);
            self.0.push(' ');
        }
    }
}

impl Visit for Features {
    fn visit_member_expr(&mut self, member: &swc_ecma_ast::MemberExpr) {
        if let Expr::Ident(object) = &*member.obj {
            match (&*object.sym, &member.prop) {
                ("Object", MemberProp::Ident(property)) => self.add(&format!("Object.{}", property.sym)),
                ("Array", MemberProp::Ident(property)) => self.add(&format!("Array.{}", property.sym)),
                ("Symbol", _) => self.add("Symbol"),
                ("Object", MemberProp::Computed(property)) => match &*property.expr {
                    Expr::Lit(swc_ecma_ast::Lit::Str(property)) => self.add(&format!("Object.{}", property.value.to_string_lossy())),
                    _ => self.add("Object["),
                },
                ("Array", MemberProp::Computed(property)) => match &*property.expr {
                    Expr::Lit(swc_ecma_ast::Lit::Str(property)) => self.add(&format!("Array.{}", property.value.to_string_lossy())),
                    _ => self.add("Array["),
                },
                _ => {}
            }
        }
        if let MemberProp::Ident(property) = &member.prop {
            if matches!(&*property.sym, "flat" | "flatMap" | "find" | "findIndex" | "includes" | "some" | "every" | "fill" | "copyWithin") {
                self.add(&format!(".{}(", property.sym));
            }
        }
        member.visit_children_with(self);
    }

    fn visit_call_expr(&mut self, call: &swc_ecma_ast::CallExpr) {
        if let Callee::Expr(callee) = &call.callee {
            if matches!(&**callee, Expr::Ident(identifier) if &*identifier.sym == "Symbol") {
                self.add("Symbol(");
            }
        }
        call.visit_children_with(self);
    }

    fn visit_for_of_stmt(&mut self, statement: &swc_ecma_ast::ForOfStmt) {
        self.add(" of ");
        statement.visit_children_with(self);
    }
    fn visit_bin_expr(&mut self, expression: &swc_ecma_ast::BinExpr) {
        if expression.op == swc_ecma_ast::BinaryOp::InstanceOf {
            self.add("Symbol.instanceOf");
        }
        expression.visit_children_with(self);
    }

    fn visit_import_decl(&mut self, import: &swc_ecma_ast::ImportDecl) {
        let source = import.src.value.to_string_lossy();
        if source.starts_with("@Java/") || source.starts_with("java:") {
            self.add("@Java/");
        }
        import.visit_children_with(self);
    }

    fn visit_assign_expr(&mut self, assign: &swc_ecma_ast::AssignExpr) {
        if let AssignTarget::Simple(SimpleAssignTarget::Member(member)) = &assign.left {
            if matches!(&member.prop, MemberProp::Computed(property) if !matches!(&*property.expr, Expr::Lit(_))) {
                self.add("Symbol");
                self.add("[");
            }
        }
        assign.visit_children_with(self);
    }
}

fn detected_features(code: &str) -> Option<String> {
    let cm = Lrc::<SourceMap>::default();
    let file = cm.new_source_file(FileName::Custom("polyfill-input.js".into()).into(), code.to_owned());
    let lexer = Lexer::new(
        Syntax::Es(EsSyntax { jsx: true, ..Default::default() }),
        swc_ecma_ast::EsVersion::EsNext,
        StringInput::from(&*file),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    let program = parser.parse_program().ok()?;
    let mut features = Features::default();
    program.visit_with(&mut features);
    Some(features.0)
}
const SOURCE: &str = include_str!("./js/polyfill.js");

fn block_after<'a>(source: &'a str, marker: &str, open: u8, close: u8) -> Option<&'a str> {
    let start = source.find(marker)?;
    let open_at = source[start..].find(open as char)? + start;
    let mut depth = 0i32;
    for (offset, byte) in source.as_bytes()[open_at..].iter().enumerate() {
        if *byte == open {
            depth += 1;
        } else if *byte == close {
            depth -= 1;
            if depth == 0 {
                return Some(&source[start..=open_at + offset]);
            }
        }
    }
    None
}

fn polyfill_call<'a>(source: &'a str, target: &str, name: &str) -> Option<&'a str> {
    let marker = format!("_polyfill({target}, \"{name}\"");
    let start = source.find(&marker)?;
    let open_at = start + "_polyfill".len();
    let mut depth = 0i32;
    for (offset, byte) in source.as_bytes()[open_at..].iter().enumerate() {
        if *byte == b'(' {
            depth += 1;
        } else if *byte == b')' {
            depth -= 1;
            if depth == 0 {
                let end = open_at + offset + 1;
                return Some(&source[start..source[end..].find(';').map(|i| end + i + 1)?]);
            }
        }
    }
    None
}

fn uses(code: &str, receiver: &str, method: &str) -> bool {
    code.contains(&format!("{receiver}.{method} ")) || code.contains(&format!("{receiver}.{method}(")) || code.contains(&format!("{receiver}["))
}

pub(crate) fn select(code: &str) -> String { let detected; let code = match detected_features(code) { Some(features) => { detected = features; detected.as_str() }, None => code };
    let dynamic_access = code.contains("Object[") || code.contains("Array[");
    let java = code.contains("@Java/") || code.contains("java:");
    let iterator = code.contains(" of ") || code.contains("Symbol.iterator");
    let symbol = iterator || code.contains("Symbol(") || code.contains("Symbol.") || code.contains("Object.from");
    let mut body = String::new();

    if symbol {
        body.push_str(block_after(SOURCE, "if (typeof global.Symbol", b'{', b'}').unwrap_or(""));
    }
    if iterator {
        body.push_str(block_after(SOURCE, "const makeIterator", b'{', b'}').unwrap_or(""));
        if let Some(call) = polyfill_call(SOURCE, "Array.prototype", "iterator") {
            body.push_str(call);
        }
        if java {
            let start = SOURCE.find("[\"java.util.List\"").unwrap_or(SOURCE.len());
            let end = SOURCE.find("// ===== Object polyfills").unwrap_or(start);
            body.push_str(&SOURCE[start..end]);
        }
    }

    for method in [
        "isJavaObject",
        "is",
        "equals",
        "from",
        "keys",
        "values",
        "entries",
        "assign",
    ] {
        if dynamic_access || uses(code, "Object", method) {
            if method == "from" {
                body.push_str(block_after(SOURCE, "function objectFrom", b'{', b'}').unwrap_or(""));
            }
            if let Some(call) = polyfill_call(SOURCE, "Object", method) {
                body.push_str(call);
            }
        }
    }
    for method in [
        "flat",
        "flatMap",
        "find",
        "findIndex",
        "includes",
        "some",
        "every",
        "fill",
        "copyWithin",
    ] {
        if dynamic_access || code.contains(&format!(".{method}(")) {
            if let Some(call) = polyfill_call(SOURCE, "Array.prototype", method) {
                body.push_str(call);
            }
        }
    }
    for method in ["from", "of"] {
        if dynamic_access || uses(code, "Array", method) {
            if let Some(call) = polyfill_call(SOURCE, "Array", method) {
                body.push_str(call);
            }
        }
    }
    if (symbol && code.contains("[")) || code.contains("Symbol.instanceOf") {
        let start = SOURCE.find("Symbol.assign =").or_else(|| SOURCE.find("Symbol.instanceOf =")).unwrap_or(SOURCE.len());
        body.push_str(&SOURCE[start..]);
    }
    body = body.replace("}_polyfill", "}\n_polyfill");
    if body.is_empty() {
        return String::new();
    }

    let java_prelude = if java || code.contains("Object.from") {
        &SOURCE[..SOURCE.find("(function (global) {").unwrap_or(0)]
    } else {
        ""
    };
    format!("/* polyfill start */\n{java_prelude}\n(function (global) {{\nconst _polyfill = (obj, key, fn) => {{ if (obj !== undefined && typeof obj[key] === \"undefined\") Object.defineProperty(obj, key, {{ value: fn, configurable: true, writable: true, enumerable: true }}); }};\n{body}\n}})(this);\n/* polyfill end */\n")
}
