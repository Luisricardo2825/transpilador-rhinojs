use std::{collections::HashMap, path::Path};

use anyhow::{bail, Context, Result};
use swc::try_with_handler;
use swc_bundler::{Bundler, Hook, Load, ModuleData, ModuleRecord};
use swc_common::{
    source_map::SourceMap,
    sync::Lrc,
    FileName, Globals, Mark, Span, GLOBALS,
};
use swc_ecma_ast::{EsVersion, KeyValueProp, Pass, Program};
use swc_ecma_loader::{resolvers::node::NodeModulesResolver, TargetEnv};

pub(crate) fn bundle(path: &Path, aliases: &[String]) -> Result<String> {
    let cm = Lrc::<SourceMap>::default();
    let globals = Globals::new();
    let entry = FileName::Real(path.canonicalize()?);
    let mut bundler = Bundler::new(
        &globals,
        cm.clone(),
        Loader {
            cm: cm.clone(),
            aliases: aliases.to_vec(),
        },
        NodeModulesResolver::without_node_modules(TargetEnv::Browser, Default::default(), false),
        Default::default(),
        Box::new(BundleHook),
    );
    let bundles = GLOBALS.set(&globals, || {
        bundler.bundle(HashMap::from([(String::from("main"), entry)]))
    })?;
    let bundle = bundles.into_iter().next().context("bundle vazio")?;
    Ok(swc::Compiler::new(cm)
        .print(&Program::Module(bundle.module), Default::default())?
        .code)
}

fn as_module(program: Program) -> swc_ecma_ast::Module {
    match program {
        Program::Module(module) => module,
        Program::Script(script) => swc_ecma_ast::Module {
            span: script.span,
            body: script
                .body
                .into_iter()
                .map(swc_ecma_ast::ModuleItem::Stmt)
                .collect(),
            shebang: script.shebang,
        },
    }
}
struct Loader {
    cm: Lrc<SourceMap>,
    aliases: Vec<String>,
}

impl Load for Loader {
    fn load(&self, file: &FileName) -> Result<ModuleData> {
        let FileName::Real(path) = file else {
            bail!("import local inválido: {file}");
        };
        let compiler = swc::Compiler::new(self.cm.clone());
        let fm = self.cm.load_file(path)?;
        let filename = path.to_string_lossy().replace('\\', "/");
        try_with_handler(self.cm.clone(), Default::default(), |handler| {
            let module = as_module(compiler.parse_js(
                fm.clone(),
                handler,
                EsVersion::EsNext,
                swc_ecma_parser::Syntax::Typescript(Default::default()),
                swc::config::IsModule::Unknown,
                None,
            )?);            let mut program = Program::Module(module);
            let unresolved_mark = Mark::new();
            let top_level_mark = Mark::new();
            compiler.run_transform(handler, false, || {
                swc_ecma_transforms_base::resolver(unresolved_mark, top_level_mark, true)
                    .process(&mut program);
                swc_ecma_transforms_typescript::strip(unresolved_mark, top_level_mark)
                    .process(&mut program);
            });
            let code = compiler.print(&program, Default::default())?.code;
            let code = crate::java_imports::rewrite_java_imports(&code, &filename, &self.aliases)?;
            let fm = self.cm.new_source_file(file.clone().into(), code);
            let module = as_module(compiler.parse_js(
                fm.clone(),
                handler,
                EsVersion::EsNext,
                swc_ecma_parser::Syntax::Es(Default::default()),
                swc::config::IsModule::Unknown,
                None,
            )?);            Ok(ModuleData {
                fm,
                module,
                helpers: swc_ecma_transforms_base::helpers::Helpers::new(false),
            })
        }).map_err(|error| anyhow::anyhow!("{error:?}"))
    }
}

struct BundleHook;

impl Hook for BundleHook {
    fn get_import_meta_props(
        &self,
        _: Span,
        _: &ModuleRecord,
    ) -> Result<Vec<KeyValueProp>> {
        Ok(vec![])
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundles_local_imports() {
        let directory = std::env::temp_dir().join(format!(
            "js2rhino-bundle-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let main = directory.join("main.ts");
        std::fs::write(
            &main,
            r#"import { Teste } from "./Teste";
new Teste();
"#,
        )
        .unwrap();
        std::fs::write(directory.join("Teste.ts"), "export class Teste {}").unwrap();

        let output = bundle(&main, &[]).unwrap();

        assert!(output.contains("class Teste"));
        assert!(!output.contains("./Teste"));
        std::fs::remove_dir_all(directory).unwrap();
    }
}
