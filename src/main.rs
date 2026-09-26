use std::{
    io::{self, Write},
    path::PathBuf,
};

use clap::Parser as ClapParser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::*;

mod bundle;
mod files;
mod java_imports;
mod polyfills;
mod transform;

/// Conversor de javascript
#[derive(ClapParser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the file or directory to read
    path: PathBuf,

    #[arg(default_value_t = String::from("dist"))]
    /// Path to out dir
    out: String,

    /// Minify output
    #[arg(short, long, default_value_t = false)]
    minify: bool,

    /// Add polyfills
    #[arg(short, long, default_value_t = false)]
    polyfill: bool,

    /// Write a .js.map file
    #[arg(long)]
    source_map: bool,

    /// Add custom import path alias
    #[arg(short, long)]
    alias: Option<String>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let m = MultiProgress::new();
    let sty = ProgressStyle::with_template(
        "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
    )
    .unwrap()
    .progress_chars("##-");

    let args: Args = Args::parse();
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

    let paths = files::get_files(args_path)?;
    let imported = files::imported_local_files(&paths)?;
    let paths: Vec<PathBuf> = paths
        .into_iter()
        .filter(|path| {
            path.canonicalize()
                .map(|path| !imported.contains(&path))
                .unwrap_or(true)
        })
        .collect();
    pb.set_length(paths.len() as u64);

    paths.par_iter().try_for_each(|path| {
        pb.set_message(path.display().to_string());
        convert(&pb, &args, Some(path))?;
        pb.inc(1);
        Ok::<(), anyhow::Error>(())
    })?;

    pb.finish_with_message("Done");
    Ok(())
}

fn convert(pb: &ProgressBar, args: &Args, path: Option<&PathBuf>) -> anyhow::Result<()> {
    let path = path.unwrap_or(&args.path);
    let aliases = files::aliases_for(path, args.alias.as_deref());
    let mut code = bundle::bundle(path, &aliases)?;
    let out_dir = PathBuf::from(&args.out);
    let relative = if args.path.is_file() {
        args.path.file_name().map(PathBuf::from).unwrap_or_default()
    } else {
        path.strip_prefix(&args.path).unwrap_or(path).to_owned()
    };
    let mut output = out_dir.join(relative);
    output.set_extension("js");
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if args.polyfill {
        code = polyfills::select(&code) + &code;
    }
    let transformed = transform::to_es3(
        &code,
        path.to_str().unwrap_or("input.ts"),
        path.extension().is_some_and(|extension| extension == "ts"),
        args.minify,
        args.source_map,
        pb,
        &aliases,
    )?;
    let code = transformed.code;
    if let Some(map) = transformed.map {
        write_file(output.with_extension("js.map").to_str().unwrap(), map)?;
    }
    write_file(output.to_str().unwrap(), code)?;
    Ok(())
}

fn write_file(file_name: &str, code: String) -> io::Result<()> {
    std::fs::File::create(file_name)?.write_all(code.as_bytes())
}
