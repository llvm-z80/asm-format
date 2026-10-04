use std::fs;
use std::path::{Path, PathBuf};

use asm_format::{Style, StyleSource, format};

fn inputs(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy();
        if path.is_dir() {
            inputs(&path, found);
        } else if !name.starts_with('.')
            && !name.contains(".expected.")
            && !name.ends_with(".toml")
            && !name.ends_with(".md")
        {
            found.push(path);
        }
    }
}

fn expected(input: &Path) -> PathBuf {
    let stem = input.file_stem().unwrap().to_string_lossy();
    let ext = input.extension().unwrap().to_string_lossy();
    input.with_file_name(format!("{stem}.expected.{ext}"))
}

fn first_difference(expected: &[u8], actual: &[u8]) -> String {
    let lines = |s: &[u8]| -> Vec<String> {
        s.split_inclusive(|&b| b == b'\n')
            .map(|l| format!("{:?}", String::from_utf8_lossy(l)))
            .collect()
    };
    let (e, a) = (lines(expected), lines(actual));
    let row = e
        .iter()
        .zip(&a)
        .position(|(x, y)| x != y)
        .unwrap_or(e.len().min(a.len()));
    let get = |v: &[String]| v.get(row).cloned().unwrap_or_else(|| "end of file".into());
    format!("line {}: expected {}, got {}", row + 1, get(&e), get(&a))
}

fn check(input: &Path) -> Result<(), String> {
    let code = fs::read(input).unwrap();
    let want = fs::read(expected(input)).map_err(|_| "missing expected file".to_string())?;
    let style = Style::find(&StyleSource::File, Some(input)).map_err(|e| e.to_string())?;
    if style.disable_format {
        return if code == want {
            Ok(())
        } else {
            Err(first_difference(&want, &code))
        };
    }
    let dialect = style.load_dialect().map_err(|e| e.to_string())?;
    let out = format(&code, &dialect, &style).map_err(|e| e.to_string())?;
    if out != want {
        return Err(first_difference(&want, &out));
    }
    let again = format(&want, &dialect, &style).map_err(|e| e.to_string())?;
    if again != want {
        return Err(format!(
            "result is not stable: {}",
            first_difference(&want, &again)
        ));
    }
    Ok(())
}

#[test]
fn cases() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cases");
    let mut found = Vec::new();
    inputs(&root, &mut found);
    found.sort();
    assert!(!found.is_empty());
    let failures: Vec<String> = found
        .iter()
        .filter_map(|input| {
            let name = input.strip_prefix(&root).unwrap().display();
            check(input).err().map(|e| format!("{name}: {e}"))
        })
        .collect();
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
