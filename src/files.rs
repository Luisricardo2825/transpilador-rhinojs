use std::{
    io,
    path::{Path, PathBuf},
};

use swc::try_with_handler;
use swc_common::{source_map::SourceMap, sync::Lrc, GLOBALS};
pub(crate) fn get_files(path: PathBuf) -> Result<Vec<PathBuf>, io::Error> {
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

pub(crate) fn aliases_for(path: &Path, explicit: Option<&str>) -> Vec<String> {
    if let Some(alias) = explicit {
        return vec![alias
            .trim_end_matches("/*")
            .trim_end_matches('/')
            .to_owned()];
    }
    let mut directory = if path.is_dir() {
        path.to_owned()
    } else {
        path.parent().unwrap_or(path).to_owned()
    };
    loop {
        let config = directory.join("tsconfig.json");
        if let Ok(text) = std::fs::read_to_string(&config) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(paths) = value
                    .pointer("/compilerOptions/paths")
                    .and_then(serde_json::Value::as_object)
                {
                    let aliases: Vec<String> = paths
                        .keys()
                        .filter_map(|key| key.strip_suffix("/*").map(str::to_owned))
                        .collect();
                    if !aliases.is_empty() {
                        return aliases;
                    }
                }
            }
        }
        if !directory.pop() {
            break;
        }
    }
    vec![]
}
pub(crate) fn imported_local_files(paths: &[PathBuf]) -> anyhow::Result<std::collections::HashSet<PathBuf>> {
    let known: std::collections::HashSet<PathBuf> = paths
        .iter()
        .map(|path| path.canonicalize())
        .collect::<io::Result<_>>()?;
    let cm = Lrc::<SourceMap>::default();
    let compiler = swc::Compiler::new(cm.clone());
    let mut imported = std::collections::HashSet::new();

    for path in paths {
        let fm = cm.load_file(path)?;
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
            continue;
        };
        for item in module.body {
            let swc_ecma_ast::ModuleItem::ModuleDecl(swc_ecma_ast::ModuleDecl::Import(import)) = item
            else {
                continue;
            };
            let source = import.src.value.as_str().unwrap_or_default();
            if !source.starts_with('.') {
                continue;
            }
            let base = path.parent().unwrap_or(path).join(source);
            for candidate in [
                base.clone(),
                base.with_extension("ts"),
                base.with_extension("tsx"),
                base.with_extension("js"),
                base.with_extension("jsx"),
                base.join("index.ts"),
                base.join("index.tsx"),
                base.join("index.js"),
                base.join("index.jsx"),
            ] {
                if let Ok(candidate) = candidate.canonicalize() {
                    if known.contains(&candidate) {
                        imported.insert(candidate);
                        break;
                    }
                }
            }
        }
    }
    Ok(imported)
}
