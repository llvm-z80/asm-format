use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use asm_format::{Dialect, Style, StyleSource};
use clap::Parser;

/// A tool to format assembly source files.
///
/// If no arguments are specified, it formats the code from standard input
/// and writes the result to the standard output.
/// If <file>s are given, it reformats the files. If -i is specified
/// together with <file>s, the files are edited in-place. Otherwise, the
/// result is written to the standard output.
#[derive(Parser)]
#[command(name = "asm-format", version)]
struct Cli {
    /// Inplace edit <file>s, if specified.
    #[arg(short = 'i')]
    inplace: bool,

    /// If set, do not actually make the formatting changes.
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// If set, changes formatting warnings to errors.
    #[arg(long = "Werror")]
    werror: bool,

    /// Set style. Use "file" to search for a style file, "file:<path>" to use
    /// the given file, or a YAML mapping such as "{Dialect: sdas}".
    #[arg(long, default_value = "file")]
    style: String,

    /// Set filename used to find the style file and to match Overrides when
    /// reading from stdin.
    #[arg(long)]
    assume_filename: Option<PathBuf>,

    /// Dump configuration options to stdout and exit.
    #[arg(long)]
    dump_config: bool,

    /// Files to format. "-" reads from stdin.
    files: Vec<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let source = match StyleSource::parse(&cli.style) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let files: Vec<Option<&Path>> = if cli.files.is_empty() {
        vec![None]
    } else {
        cli.files
            .iter()
            .map(|f| (f.as_os_str() != "-").then_some(f.as_path()))
            .collect()
    };
    if cli.dump_config {
        let path = files[0].or(cli.assume_filename.as_deref());
        return match Style::find(&source, path) {
            Ok(style) => {
                print!("{}", style.dump());
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        };
    }
    if cli.inplace && files.iter().any(Option::is_none) {
        eprintln!("error: cannot use -i when reading from stdin.");
        return ExitCode::FAILURE;
    }
    let mut dialects = HashMap::new();
    let mut ok = true;
    for file in files {
        ok &= run(&cli, &source, file, &mut dialects);
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

type Dialects = HashMap<(PathBuf, String), Dialect>;

fn run(cli: &Cli, source: &StyleSource, file: Option<&Path>, dialects: &mut Dialects) -> bool {
    let name = file.map_or_else(|| "<stdin>".to_string(), |f| f.display().to_string());
    match format(cli, source, file, dialects) {
        Ok((input, output)) => emit(cli, &name, file, &input, &output),
        Err(e) => {
            eprintln!("{name}: error: {e}");
            false
        }
    }
}

fn format(
    cli: &Cli,
    source: &StyleSource,
    file: Option<&Path>,
    dialects: &mut Dialects,
) -> Result<(Vec<u8>, Vec<u8>), String> {
    let input = match file {
        Some(f) => std::fs::read(f).map_err(|e| e.to_string())?,
        None => {
            let mut buf = Vec::new();
            std::io::stdin()
                .read_to_end(&mut buf)
                .map_err(|e| e.to_string())?;
            buf
        }
    };
    let style =
        Style::find(source, file.or(cli.assume_filename.as_deref())).map_err(|e| e.to_string())?;
    if style.disable_format {
        return Ok((input.clone(), input));
    }
    let key = (
        style.dialect_dir.clone(),
        style.dialect.clone().unwrap_or_default(),
    );
    let dialect = match dialects.entry(key) {
        Entry::Occupied(e) => e.into_mut(),
        Entry::Vacant(e) => e.insert(style.load_dialect().map_err(|e| e.to_string())?),
    };
    let output = asm_format::format(&input, dialect, &style).map_err(|e| e.to_string())?;
    Ok((input, output))
}

fn emit(cli: &Cli, name: &str, file: Option<&Path>, input: &[u8], output: &[u8]) -> bool {
    if cli.dry_run {
        if input == output {
            return true;
        }
        report(name, input, output, cli.werror);
        return !cli.werror;
    }
    if cli.inplace {
        if input != output
            && let Some(f) = file
            && let Err(e) = std::fs::write(f, output)
        {
            eprintln!("{name}: error: {e}");
            return false;
        }
        return true;
    }
    if let Err(e) = std::io::stdout().write_all(output) {
        eprintln!("error: {e}");
        return false;
    }
    true
}

fn report(name: &str, input: &[u8], output: &[u8], werror: bool) {
    let a: Vec<&[u8]> = input.split(|&b| b == b'\n').collect();
    let b: Vec<&[u8]> = output.split(|&b| b == b'\n').collect();
    let row = a
        .iter()
        .zip(&b)
        .position(|(x, y)| x != y)
        .unwrap_or(a.len().min(b.len()));
    let line = a.get(row).copied().unwrap_or_default();
    let other = b.get(row).copied().unwrap_or_default();
    let col = line
        .iter()
        .zip(other)
        .position(|(x, y)| x != y)
        .unwrap_or(line.len().min(other.len()));
    let level = if werror { "error" } else { "warning" };
    eprintln!(
        "{name}:{}:{}: {level}: code should be asm-formatted [-Wasm-format-violations]",
        row + 1,
        col + 1
    );
    eprintln!("{}", String::from_utf8_lossy(line).trim_end_matches('\r'));
    eprintln!("{}^", " ".repeat(col));
}
