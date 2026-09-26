use swc::try_with_handler;
use swc_common::{source_map::SourceMap, sync::Lrc, FileName, GLOBALS};
pub(crate) fn rewrite_java_imports(
    code: &str,
    filename: &str,
    aliases: &[String],
) -> anyhow::Result<String> {
    let cm = Lrc::<SourceMap>::default();
    let compiler = swc::Compiler::new(cm.clone());
    let fm = cm.new_source_file(FileName::Custom(filename.into()).into(), code.to_owned());
    let program = GLOBALS
        .set(&Default::default(), || {
            try_with_handler(cm.clone(), Default::default(), |handler| {
                compiler.parse_js(
                    fm,
                    handler,
                    swc_ecma_ast::EsVersion::EsNext,
                    swc_ecma_parser::Syntax::Typescript(Default::default()),
                    swc::config::IsModule::Unknown,
                    None,
                )
            })
        })
        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
    let swc_ecma_ast::Program::Module(module) = program else {
        return Ok(code.to_owned());
    };

    let mut replacements = Vec::new();
    for item in module.body {
        let swc_ecma_ast::ModuleItem::ModuleDecl(swc_ecma_ast::ModuleDecl::Import(import)) = item
        else {
            continue;
        };
        let source = import.src.value.as_str().unwrap().to_string();
        let Some(alias) = aliases
            .iter()
            .find(|alias| source.starts_with(alias.as_str()))
            .map(String::as_str)
            .or_else(|| is_java_import(&source).then_some(""))
        else {
            continue;
        };
        if import.type_only {
            replacements.push((
                import.span.lo.0 as usize - 1,
                import.span.hi.0 as usize - 1,
                String::new(),
            ));
            continue;
        }
        let reference = java_reference(&source, alias);
        let is_side_effect = import.specifiers.is_empty();
        let mut declarations = Vec::new();
        for specifier in import.specifiers {
            if specifier.is_type_only() {
                continue;
            }
            match specifier {
                swc_ecma_ast::ImportSpecifier::Default(specifier) => {
                    declarations.push(format!("const {} = {reference};", specifier.local.sym))
                }
                swc_ecma_ast::ImportSpecifier::Namespace(specifier) => {
                    declarations.push(format!("const {} = {reference};", specifier.local.sym))
                }
                swc_ecma_ast::ImportSpecifier::Named(specifier) => {
                    let imported = match specifier.imported {
                        Some(swc_ecma_ast::ModuleExportName::Ident(name)) => name.sym.to_string(),
                        Some(swc_ecma_ast::ModuleExportName::Str(name)) => {
                            format!("[{:?}]", name.value)
                        }
                        None => specifier.local.sym.to_string(),
                    };
                    let access = if imported.starts_with('[') {
                        imported
                    } else if alias.is_empty()
                        && java_class_name(&source, alias)
                            .is_some_and(|class_name| class_name == imported)
                    {
                        String::new()
                    } else {
                        format!(".{imported}")
                    };
                    declarations.push(format!(
                        "const {} = {reference}{access};",
                        specifier.local.sym
                    ));
                }
            }
        }
        if is_side_effect {
            if let Some(class_name) = java_class_name(&source, alias) {
                declarations.push(format!("const {class_name} = {reference};"));
            }
        }
        replacements.push((
            import.span.lo.0 as usize - 1,
            import.span.hi.0 as usize - 1,
            declarations.join("\n"),
        ));
    }
    let mut output = code.to_owned();
    replacements.sort_by_key(|(start, _, _)| *start);
    for (start, end, replacement) in replacements.into_iter().rev() {
        output.replace_range(start..end, &replacement);
    }
    Ok(output)
}

fn is_java_import(source: &str) -> bool {
    ["java:", "java.", "javax.", "com.", "org.", "net.", "br."]
        .iter()
        .any(|prefix| source.starts_with(prefix))
}
fn java_class_name<'a>(source: &'a str, alias: &str) -> Option<&'a str> {
    let class_path = source
        .strip_prefix(alias)
        .unwrap_or(source)
        .trim_start_matches('/')
        .trim_start_matches("java:");
    let name = class_path.rsplit('/').next()?.rsplit('.').next()?;
    let mut characters = name.chars();
    match characters.next()? {
        'a'..='z' | 'A'..='Z' | '_' | '$' => {}
        _ => return None,
    }
    characters
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '$'))
        .then_some(name)
}
fn java_reference(source: &str, alias: &str) -> String {
    let class_path = source
        .strip_prefix(alias)
        .unwrap_or(source)
        .trim_start_matches('/')
        .trim_start_matches("java:")
        .trim()
        .replace('/', ".");
    if class_path.to_ascii_lowercase().starts_with("java.") {
        class_path
    } else {
        format!("Packages.{class_path}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_all_static_java_import_forms() {
        let output = rewrite_java_imports(
            r#"
                import DefaultClass from "@Java/br/com/example/DefaultClass";
                import * as Example from "@Java/br/com/example";
                import { Foo, Bar as LocalBar, type TypeOnly } from "@Java/br/com/example";
                import JavaString from "@Java/java/lang/String";
                import "@Java/br/com/example/SideEffect";
                import type { CompileOnly } from "@Java/br/com/example";
                import "br.com.sankhya.jape.EntityFacade";
                import { JsonObject } from "com.google.gson";
                import { AbstractMap } from "java.util";
                import { ServiceContext } from "br.com.sankhya.ws.ServiceContext";
            "#,
            "imports.test.ts",
            &["@Java".to_owned()],
        )
        .unwrap();

        assert!(output.contains("const DefaultClass = Packages.br.com.example.DefaultClass;"));
        assert!(output.contains("const Example = Packages.br.com.example;"));
        assert!(output.contains("const Foo = Packages.br.com.example.Foo;"));
        assert!(output.contains("const LocalBar = Packages.br.com.example.Bar;"));
        assert!(output.contains("const JavaString = java.lang.String;"));
        assert!(output.contains("const SideEffect = Packages.br.com.example.SideEffect;"));
        assert!(!output.contains("TypeOnly"));
        assert!(!output.contains("CompileOnly"));
        assert!(output.contains("const EntityFacade = Packages.br.com.sankhya.jape.EntityFacade;"));
        assert!(output.contains("const JsonObject = Packages.com.google.gson.JsonObject;"));
        assert!(output.contains("const AbstractMap = java.util.AbstractMap;"));
        assert!(
            output.contains("const ServiceContext = Packages.br.com.sankhya.ws.ServiceContext;")
        );
    }
}
