use clap::Parser as OtherParser;
use pest::Parser;
use std::{
    io::{self, Write},
    path::PathBuf,
};

/// Conversor de javascript
#[derive(OtherParser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the file or directory to read
    path: std::path::PathBuf,

    #[arg(default_value_t = String::from("dist"))]
    /// Path to out dir
    out: String,
}

#[derive(pest_derive::Parser)]
#[grammar = "grammar.pest"]
pub struct LangParser;

fn main() -> io::Result<()> {
    let args: Args = Args::parse();

    if !args.path.is_dir() {
        convert(args.path, args.out)?;

        return Ok(());
    }

    let mut paths = vec![];
    for entry in std::fs::read_dir(args.path.clone())? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        paths.push(path);
    }
    for path in paths {
        convert(path, args.out.clone())?;
    }
    Ok(())
}

fn convert(mut path: PathBuf, out: String) -> Result<(), io::Error> {
    let unparsed_file = std::fs::read_to_string(path.clone()).expect("Erro reading file");
    let pairs = LangParser::parse(Rule::program, &unparsed_file).expect("Erro parsing");
    let mut code = vec![];
    let default_imports = vec![[
        Values::String("System".to_owned()),
        Values::String("java.lang.System".to_owned()),
    ]];

    for ele in default_imports {
        let result = resolve(&ele[0], &ele[1].to_string()).to_string();

        code.push(result + ";");
    }
    let size = pairs.len();
    for (i, pair) in pairs.enumerate() {
        let str = pair.as_str();
        if str.len() <= 0 {
            continue;
        }
        let result = resolve_rule(pair);

        // End of file
        if i < size - 1 {
            code.push(result.to_string());
            continue;
        }

        code.push(result.to_string() + ";");
    }

    let code = code.join("\n");
    path.set_extension("js");
    let file_name = path.file_name().unwrap().to_str().unwrap();
    // Create out dir
    std::fs::create_dir_all(format!("./{}", out)).expect("Erro ao criar pasta de saida");
    // Create out file
    let mut out =
        std::fs::File::create(format!("{}/{}", out, file_name)).expect("Erro ao criar arquivo");
    out.write_all(code.as_bytes())
        .expect("Erro ao escrever arquivo");
    out.flush()?;
    Ok(())
}

#[derive(Debug, Clone)]
enum Values {
    String(String),
    Mutiple(Vec<Values>),
    Rename {
        old: String,
        new: String,
    },
    DefaultWithMutiple {
        mutiple: Vec<Values>,
        default: String,
    },
}
fn resolve_rule(primary: pest::iterators::Pair<'_, Rule>) -> Values {
    match primary.as_rule() {
        Rule::import => {
            let mut pair = primary.into_inner();
            let vars = pair.next().unwrap();
            let source = pair.next().unwrap();
            let var = resolve_rule(vars);
            let source = resolve_rule(source);
            let source = match source {
                Values::String(val) => val,
                Values::Mutiple(_) => todo!(),
                Values::Rename { .. } => todo!(),
                Values::DefaultWithMutiple { .. } => todo!(),
            };

            let result = resolve(&var, &source);

            Values::String(result)
        }
        Rule::ident => Values::String(String::from(primary.as_str())),
        Rule::source => {
            let str = primary.as_str().to_owned();
            let str = &str[1..str.len() - 1];

            let str = str.replace("\\\\", "\\");
            let str = str.replace("\\\"", "\"");
            let str = str.replace("\\n", "\n");
            let str = str.replace("\\r", "\r");
            let str = str.replace("\\t", "\t");
            Values::String(str)
        }
        Rule::rename => {
            let mut pair = primary.into_inner();
            let past_name = pair.next().unwrap();
            let name = pair.next().unwrap();
            Values::Rename {
                new: name.as_str().to_string(),
                old: past_name.as_str().to_string(),
            }
        }
        Rule::destructuring => {
            let mut pair = primary.into_inner();
            let mut idents = vec![];
            loop {
                let ident = pair.next();
                if ident.is_none() {
                    break;
                }
                let ident = ident.unwrap();
                let ident = resolve_rule(ident);
                idents.push(ident)
            }
            Values::Mutiple(idents)
        }
        Rule::defaultWithDestructuring => {
            let mut pair = primary.into_inner();
            let ident = pair.next().unwrap();
            let destruct = pair.next().unwrap();
            let destruct = resolve_rule(destruct).to_multiple();

            Values::DefaultWithMutiple {
                default: ident.as_str().to_string(),
                mutiple: destruct,
            }
        }
        Rule::directImport => {
            let mut pair = primary.into_inner();
            let source = pair.next().unwrap();
            let source = resolve_rule(source);
            let source = match source {
                Values::String(val) => val,
                Values::Mutiple(_) => todo!(),
                Values::Rename { .. } => todo!(),
                Values::DefaultWithMutiple { .. } => todo!(),
            };
            let name = source.split(".").last().unwrap();
            if source.trim().to_lowercase().starts_with("java.") {
                return Values::String(format!("const {name} = {}", source));
            }
            Values::String(format!("const {name} = Packages.{source}"))
        }
        Rule::rest => Values::String(String::from(primary.as_str())),
        rule => unreachable!("Expr::parse expected atom, found {:?}", rule),
    }
}

fn resolve(var: &Values, source: &String) -> String {
    if !source.contains(".") {
        return "".to_owned();
    }
    match var {
        Values::String(varname) => {
            if source.trim().to_lowercase().starts_with("java.") {
                return format!("const {varname} = {source}");
            }
            format!("const {} = Packages.{}", varname, source)
        }
        Values::Mutiple(vars) => {
            let mut result = String::new();
            for (i, var) in vars.iter().enumerate() {
                if var.is_rename() {
                    let (old, new) = var.to_rename();
                    if source.trim().to_lowercase().starts_with("java.") {
                        result.push_str(&format!("const {new} = {source}.{old}"));
                        continue;
                    }
                    result.push_str(&format!("const {new} = Packages.{source}.{old}"));
                    continue;
                }

                if source.trim().to_lowercase().starts_with("java.") {
                    result.push_str(&format!(
                        "const {} = {}.{}",
                        var.to_string(),
                        source,
                        var.to_string()
                    ));
                } else {
                    result.push_str(&format!(
                        "const {} = Packages.{}.{}",
                        var.to_string(),
                        source,
                        var.to_string()
                    ));
                }

                if i < vars.len() - 1 {
                    result.push_str("\n");
                }
            }
            result
        }
        Values::Rename { new, old } => {
            if source.trim().to_lowercase().starts_with("java.") {
                format!("const {} = Packages.{}.{}", new, source, old)
            } else {
                format!("const {new} = {source}.{old}")
            }
        }
        Values::DefaultWithMutiple { mutiple, default } => {
            let first = resolve(&Values::String(default.to_string()), source);
            let results = resolve(&Values::Mutiple(mutiple.to_owned()), source);
            let mut result = String::new();

            result.push_str(&first);
            result.push_str("\n");
            result.push_str(&results);

            result
        }
    }
}

impl Values {
    fn to_string(&self) -> String {
        match self {
            Values::String(val) => val.to_owned(),
            Values::Mutiple(vals) => {
                let mut result = String::new();
                for (i, val) in vals.iter().enumerate() {
                    result.push_str(&val.to_string());
                    if i < vals.len() - 1 {
                        result.push_str(", ");
                    }
                }
                result
            }
            Values::Rename { new, old } => format!("{} as {}", new, old),
            Values::DefaultWithMutiple { mutiple, default } => {
                let mut result = String::new();
                for (i, val) in mutiple.iter().enumerate() {
                    result.push_str(&val.to_string());
                    if i < mutiple.len() - 1 {
                        result.push_str(", ");
                    }
                }
                format!("{} as {}", result, default)
            }
        }
    }
    fn is_rename(&self) -> bool {
        match self {
            Values::Rename { .. } => true,
            _ => false,
        }
    }
    fn to_rename(&self) -> (String, String) {
        match self {
            Values::Rename { old, new } => (old.to_string(), new.to_string()),
            _ => panic!("Not a rename"),
        }
    }
    fn to_multiple(&self) -> Vec<Values> {
        match self {
            Values::Mutiple(vals) => vals.to_owned(),
            _ => panic!("Not a multiple"),
        }
    }
}
