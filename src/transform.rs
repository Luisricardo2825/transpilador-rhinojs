use anyhow::Context;
use indicatif::ProgressBar;
use swc::try_with_handler;
use swc_ecma_ast::Pass;
use swc_ecma_visit::VisitMutWith;
use swc_common::{source_map::SourceMap, sync::Lrc, FileName, Mark, GLOBALS};

use crate::java_imports::rewrite_java_imports;
/// Transforms typescript to javascript. Returns tuple (js string, source map)
fn swc_filename(filename: &str) -> String {
    filename.replace('\\', "/")
}
struct ComputedSymbolAssignments;

impl swc_ecma_visit::VisitMut for ComputedSymbolAssignments {
    fn visit_mut_expr(&mut self, expression: &mut swc_ecma_ast::Expr) {
        expression.visit_mut_children_with(self);
        let swc_ecma_ast::Expr::Assign(assign) = expression else {
            return;
        };
        let (method, operator) = match assign.op {
            swc_ecma_ast::AssignOp::Assign => ("set", None),
            swc_ecma_ast::AssignOp::AddAssign => ("assign", Some("+=")),
            swc_ecma_ast::AssignOp::SubAssign => ("assign", Some("-=")),
            swc_ecma_ast::AssignOp::MulAssign => ("assign", Some("*=")),
            swc_ecma_ast::AssignOp::DivAssign => ("assign", Some("/=")),
            swc_ecma_ast::AssignOp::ModAssign => ("assign", Some("%=")),
            swc_ecma_ast::AssignOp::ExpAssign => ("assign", Some("**=")),
            swc_ecma_ast::AssignOp::LShiftAssign => ("assign", Some("<<=")),
            swc_ecma_ast::AssignOp::RShiftAssign => ("assign", Some(">>=")),
            swc_ecma_ast::AssignOp::ZeroFillRShiftAssign => ("assign", Some(">>>=")),
            swc_ecma_ast::AssignOp::BitOrAssign => ("assign", Some("|=")),
            swc_ecma_ast::AssignOp::BitXorAssign => ("assign", Some("^=")),
            swc_ecma_ast::AssignOp::BitAndAssign => ("assign", Some("&=")),
            swc_ecma_ast::AssignOp::AndAssign => ("assign", Some("&&=")),
            swc_ecma_ast::AssignOp::OrAssign => ("assign", Some("||=")),
            swc_ecma_ast::AssignOp::NullishAssign => ("assign", Some("??=")),
        };
        let swc_ecma_ast::AssignTarget::Simple(
            swc_ecma_ast::SimpleAssignTarget::Member(member),
        ) = &assign.left else {
            return;
        };
        let member = member.clone();
        let swc_ecma_ast::MemberProp::Computed(property) = member.prop else {
            return;
        };
        if matches!(&*property.expr, swc_ecma_ast::Expr::Lit(_)) {
            return;
        }
        let direct_rhs = matches!(&*assign.right, swc_ecma_ast::Expr::Lit(swc_ecma_ast::Lit::Str(_) | swc_ecma_ast::Lit::Bool(_) | swc_ecma_ast::Lit::Null(_) | swc_ecma_ast::Lit::Num(_) | swc_ecma_ast::Lit::BigInt(_)));

        let mut args = vec![
            swc_ecma_ast::ExprOrSpread { spread: None, expr: member.obj },
            swc_ecma_ast::ExprOrSpread { spread: None, expr: property.expr },
        ];
        if let Some(operator) = operator {
            args.push(swc_ecma_ast::ExprOrSpread {
                spread: None,
                expr: Box::new(swc_ecma_ast::Expr::Lit(swc_ecma_ast::Lit::Str(swc_ecma_ast::Str {
                    span: swc_common::DUMMY_SP,
                    value: operator.into(),
                    raw: None,
                }))),
            });
            if direct_rhs {
                args.push(swc_ecma_ast::ExprOrSpread {
                    spread: None,
                    expr: assign.right.clone(),
                });
            } else {
                // Compound assignments read LHS before evaluating RHS.
                args.push(swc_ecma_ast::ExprOrSpread {
                    spread: None,
                    expr: Box::new(swc_ecma_ast::Expr::Arrow(swc_ecma_ast::ArrowExpr {
                        span: swc_common::DUMMY_SP,
                        ctxt: swc_common::SyntaxContext::empty(),
                        params: Vec::new(),
                        body: Box::new(swc_ecma_ast::ArrowFunctionBody::Expr(assign.right.clone())),
                        is_async: false,
                        is_generator: false,
                        type_params: None,
                        return_type: None,
                    })),
                });
            }
        } else {
            args.push(swc_ecma_ast::ExprOrSpread { spread: None, expr: assign.right.clone() });
        }

        *expression = swc_ecma_ast::Expr::Call(swc_ecma_ast::CallExpr {
            span: assign.span,
            ctxt: swc_common::SyntaxContext::empty(),
            callee: swc_ecma_ast::Callee::Expr(Box::new(swc_ecma_ast::Expr::Member(swc_ecma_ast::MemberExpr {
                span: swc_common::DUMMY_SP,
                obj: Box::new(swc_ecma_ast::Expr::Ident(swc_ecma_ast::Ident::new_no_ctxt("Symbol".into(), swc_common::DUMMY_SP))),
                prop: swc_ecma_ast::MemberProp::Ident(swc_ecma_ast::IdentName::new(method.into(), swc_common::DUMMY_SP)),
            }))),
            args,
            type_args: None,
        });
    }
}
struct CustomInstanceOf;

impl swc_ecma_visit::VisitMut for CustomInstanceOf {
    fn visit_mut_expr(&mut self, expression: &mut swc_ecma_ast::Expr) {
        expression.visit_mut_children_with(self);
        let swc_ecma_ast::Expr::Bin(binary) = expression else {
            return;
        };
        if binary.op != swc_ecma_ast::BinaryOp::InstanceOf {
            return;
        };
        *expression = swc_ecma_ast::Expr::Call(swc_ecma_ast::CallExpr {
            span: binary.span,
            ctxt: swc_common::SyntaxContext::empty(),
            callee: swc_ecma_ast::Callee::Expr(Box::new(swc_ecma_ast::Expr::Member(
                swc_ecma_ast::MemberExpr {
                    span: swc_common::DUMMY_SP,
                    obj: Box::new(swc_ecma_ast::Expr::Ident(
                        swc_ecma_ast::Ident::new_no_ctxt("Symbol".into(), swc_common::DUMMY_SP),
                    )),
                    prop: swc_ecma_ast::MemberProp::Ident(
                        swc_ecma_ast::IdentName::new("instanceOf".into(), swc_common::DUMMY_SP),
                    ),
                },
            ))),
            args: vec![
                swc_ecma_ast::ExprOrSpread { spread: None, expr: binary.left.clone() },
                swc_ecma_ast::ExprOrSpread { spread: None, expr: binary.right.clone() },
            ],
            type_args: None,
        });
    }
}
pub(crate) struct TransformOutput { pub(crate) code: String, pub(crate) map: Option<String> }

pub(crate) fn to_es3(
    code: &str,
    filename: &str,
    is_typescript: bool,
    minify: bool,
    source_map: bool,
    pb: &ProgressBar,
    aliases: &[String],
) -> anyhow::Result<TransformOutput> {
    let cm = Lrc::<SourceMap>::default();
    let compiler = swc::Compiler::new(cm.clone());
    let filename = swc_filename(filename);
    let output = GLOBALS.set(&Default::default(), || {
        try_with_handler(cm.clone(), Default::default(), |handler| {
            let fm = cm.new_source_file(
                FileName::Custom(filename.clone().into()).into(),
                code.to_owned(),
            );
            if is_typescript {
                pb.set_message(format!("Compiling {filename}"));
                pb.inc(1);
            }
            let program = compiler
                .parse_js(
                    fm,
                    handler,
                    swc_ecma_ast::EsVersion::Es5,
                    swc_ecma_parser::Syntax::Typescript(Default::default()),
                    swc::config::IsModule::Unknown,
                    None,
                )
                .context(format!("failed to parse file {}", filename))?;

            let strip_unresolved_mark = Mark::new();
            let strip_top_level_mark = Mark::new();
            let stripped = compiler.run_transform(handler, false, || {
                let mut program = program;
                swc_ecma_transforms_base::resolver(
                    strip_unresolved_mark,
                    strip_top_level_mark,
                    true,
                )
                .process(&mut program);
                swc_ecma_transforms_typescript::strip(strip_unresolved_mark, strip_top_level_mark)
                    .process(&mut program);
                program
            });
            let stripped_code = compiler.print(&stripped, Default::default())?.code;
            let rewritten = rewrite_java_imports(&stripped_code, &filename, aliases)?;
            let fm =
                cm.new_source_file(FileName::Custom(filename.clone().into()).into(), rewritten);
            let program = compiler
                .parse_js(
                    fm,
                    handler,
                    swc_ecma_ast::EsVersion::Es5,
                    swc_ecma_parser::Syntax::Es(Default::default()),
                    swc::config::IsModule::Unknown,
                    None,
                )
                .context(format!("failed to parse rewritten file {}", filename))?;
            let unresolved_mark = Mark::new();
            let top_level_mark = Mark::new();
            let program = compiler.run_transform(handler, false, || {
                let mut program = program;
                swc_ecma_transforms_base::resolver(unresolved_mark, top_level_mark, true)
                    .process(&mut program);
                program.visit_mut_with(&mut ComputedSymbolAssignments);
                program.visit_mut_with(&mut CustomInstanceOf);
                swc_ecma_preset_env::transform_from_es_version(
                    unresolved_mark,
                    None::<swc_common::comments::SingleThreadedComments>,
                    swc_ecma_ast::EsVersion::Es5,
                    swc_ecma_transforms_base::assumptions::Assumptions::default(),
                    false,
                )
                .process(&mut program);
                swc_ecma_transforms_base::helpers::inject_helpers(top_level_mark)
                    .process(&mut program);
                swc_ecma_transforms_base::fixer::fixer(None).process(&mut program);
                program
            });
            let output = compiler.print(&program, swc::PrintArgs { source_file_name: Some(&filename), inline_sources_content: true, source_map: swc::config::SourceMapsConfig::Bool(source_map), ..Default::default() })?;

            if minify {
                let minify_options: swc::config::JsMinifyOptions = serde_json::from_str(r#"{"compress":true,"mangle":true}"#)?;
            let code = compiler
                    .minify(
                        cm.new_source_file(
                            FileName::Custom("output.js".into()).into(),
                            output.code,
                        ),
                        handler,
                        &minify_options,
                    Default::default(),
                    )?
                    .code; Ok(TransformOutput { code, map: None })
            } else {
                Ok(TransformOutput { code: output.code, map: output.map })
            }
        })
    });

    output.map_err(|error| anyhow::anyhow!("{error}"))
}

