use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

use anyhow::Context;
use clap::Parser as ClapParser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use swc::try_with_handler;
use swc_ecma_ast::Pass;

use swc_common::{
    source_map::SourceMap,
    sync::{Lazy, Lrc},
    FileName, Mark, GLOBALS,
};

/// Conversor de javascript
#[derive(ClapParser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the file or directory to read
    path: std::path::PathBuf,

    #[arg(default_value_t = String::from("dist"))]
    /// Path to out dir
    out: String,

    #[arg(short, long, default_value_t = false)]
    /// Minify the output
    minify: bool,

    /// Add polyfills
    #[arg(short, long, default_value_t = false)]
    polyfill: bool,

    /// Add custom import path alias
    #[arg(short, long, default_value_t = String::from("@Java/"))]
    alias: String,
}

static ALIAS: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

// Get /js/pollyfill.js file
fn get_polyfill() -> String {
    let pollyfill = include_str!("./js/pollyfill.js");
    format!("/*pollyfill start*/ {} /*pollyfill end*/\n", pollyfill)
}

fn main() -> io::Result<()> {
    let m = MultiProgress::new();
    let sty = ProgressStyle::with_template(
        "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
    )
    .unwrap()
    .progress_chars("##-");

    let args: Args = Args::parse();

    *ALIAS.lock().unwrap() = Some(args.alias.clone());
    let start = std::time::Instant::now();

    let n = 200;
    let pb: ProgressBar = m.add(ProgressBar::new(n));
    pb.set_style(sty.clone());

    let args_path = args.path.clone();

    if !args.path.is_dir() {
        pb.set_length(1);
        pb.inc(1);
        convert(&pb, &args, None)?;
        pb.finish_with_message("Done");
        println!("Time: {:?}", start.elapsed());

        return Ok(());
    }

    let paths: Vec<PathBuf> = get_files(args_path)?;
    pb.set_length(paths.len() as u64);

    paths.par_iter().try_for_each(|path| {
        pb.set_message(path.display().to_string());
        convert(&pb, &args, Some(path))?;
        pb.inc(1);
        Ok::<(), io::Error>(())
    })?;

    pb.finish_with_message("Done");
    Ok(())
}

fn get_files(path: PathBuf) -> Result<Vec<PathBuf>, io::Error> {
    let mut paths = vec![];
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            // Get sub folders
            let sub_paths = get_files(path)?;
            paths.extend(sub_paths);
            continue;
        }

        // Check if is js or ts file
        if !matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("js" | "ts")
        ) {
            continue;
        }
        // Run the conver concurrently
        paths.push(path)
    }
    Ok(paths)
}

fn convert(pb: &ProgressBar, args: &Args, path: Option<&PathBuf>) -> Result<(), io::Error> {
    let out = &args.out;
    let path = match path {
        Some(path) => path,
        None => &args.path,
    };
    let input_base = args.path.clone();
    let minify = args.minify;
    let polyfills = args.polyfill;
    let unparsed_file = std::fs::read_to_string(path.clone())?;
    let mut code = unparsed_file;
    // cria pasta de saída
    let out_dir = PathBuf::from(out);

    // calcula o caminho relativo em relação à pasta de entrada
    let relative = if input_base.is_file() {
        input_base
            .file_name()
            .map(PathBuf::from)
            .unwrap_or_default()
    } else {
        path.strip_prefix(input_base).unwrap_or(path).to_owned()
    };

    // monta o caminho final
    let mut out_filename = out_dir.join(relative);

    // força extensão para .js
    out_filename.set_extension("js");

    // cria diretórios se necessário
    if let Some(parent) = out_filename.parent() {
        std::fs::create_dir_all(parent)?;
    }

    if polyfills {
        let pollyfill = get_polyfill();

        code = pollyfill + code.as_str();
    }
    write_file(out_filename.to_str().unwrap(), code)?;

    // compila para es3
    let code = to_es3(
        out_filename.to_str().unwrap(),
        path.extension().is_some_and(|extension| extension == "ts"),
        minify,
        pb,
    )
    .map_err(io::Error::other)?;

    write_file(out_filename.to_str().unwrap(), code)?;

    Ok(())
}

fn write_file(file_name: &str, code: String) -> io::Result<()> {
    std::fs::File::create(file_name)?.write_all(code.as_bytes())
}
fn rewrite_java_imports(code: &str) -> anyhow::Result<String> {
    let cm = Lrc::<SourceMap>::default();
    let compiler = swc::Compiler::new(cm.clone());
    let fm = cm.new_source_file(FileName::Custom("input.ts".into()).into(), code.to_owned());
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
    let alias = ALIAS.lock().unwrap().clone().unwrap_or_default();
    let mut replacements = Vec::new();
    for item in module.body {
        let swc_ecma_ast::ModuleItem::ModuleDecl(swc_ecma_ast::ModuleDecl::Import(import)) = item
        else {
            continue;
        };
        let source = import.src.value.as_str().unwrap().to_string();
        if !source.starts_with(&alias) {
            continue;
        }
        if import.type_only {
            replacements.push((
                import.span.lo.0 as usize - 1,
                import.span.hi.0 as usize - 1,
                String::new(),
            ));
            continue;
        }
        let reference = java_reference(&source, &alias);
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
        if declarations.is_empty() {
            declarations.push(format!("{reference};"));
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

fn java_reference(source: &str, alias: &str) -> String {
    let class_path = source
        .strip_prefix(alias)
        .unwrap_or(source)
        .trim()
        .replace('/', ".");
    if class_path.to_ascii_lowercase().starts_with("java.") {
        class_path
    } else {
        format!("Packages.{class_path}")
    }
}

/// Transforms typescript to javascript. Returns tuple (js string, source map)
fn to_es3(
    filename: &str,
    is_typescript: bool,
    minify: bool,
    pb: &ProgressBar,
) -> anyhow::Result<String> {
    let cm = Lrc::<SourceMap>::default();
    let compiler = swc::Compiler::new(cm.clone());
    let output = GLOBALS.set(&Default::default(), || {
        try_with_handler(cm.clone(), Default::default(), |handler| {
            let fm = cm
                .load_file(Path::new(filename))
                .expect("failed to load file");
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
            let rewritten = rewrite_java_imports(&stripped_code)?;
            let fm = cm.new_source_file(FileName::Custom(filename.into()).into(), rewritten);
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
            let output = compiler.print(&program, Default::default())?;

            if minify {
                Ok(compiler
                    .minify(
                        cm.new_source_file(
                            FileName::Custom("output.js".into()).into(),
                            output.code,
                        ),
                        handler,
                        &Default::default(),
                        Default::default(),
                    )?
                    .code)
            } else {
                Ok(output.code)
            }
        })
    });

    output.map_err(|error| anyhow::anyhow!("Error processing {filename}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_all_static_java_import_forms() {
        *ALIAS.lock().unwrap() = Some("@Java/".to_owned());
        let output = rewrite_java_imports(
            r#"
                import DefaultClass from "@Java/br/com/example/DefaultClass";
                import * as Example from "@Java/br/com/example";
                import { Foo, Bar as LocalBar, type TypeOnly } from "@Java/br/com/example";
                import JavaString from "@Java/java/lang/String";
                import "@Java/br/com/example/SideEffect";
                import type { CompileOnly } from "@Java/br/com/example";
            "#,
        )
        .unwrap();

        assert!(output.contains("const DefaultClass = Packages.br.com.example.DefaultClass;"));
        assert!(output.contains("const Example = Packages.br.com.example;"));
        assert!(output.contains("const Foo = Packages.br.com.example.Foo;"));
        assert!(output.contains("const LocalBar = Packages.br.com.example.Bar;"));
        assert!(output.contains("const JavaString = java.lang.String;"));
        assert!(output.contains("Packages.br.com.example.SideEffect;"));
        assert!(!output.contains("TypeOnly"));
        assert!(!output.contains("CompileOnly"));
    }
}
