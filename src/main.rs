use clap::{Arg, ArgAction, Command};
use indexmap::IndexMap;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

fn read_json(path: &Path) -> Result<Value, String> {
    fs::read_to_string(path)
        .map_err(|e| format!("read {}: {}", path.display(), e))
        .and_then(|s| {
            serde_json::from_str(&s).map_err(|e| format!("parse {}: {}", path.display(), e))
        })
}

fn keys_from_value(v: &Value) -> Vec<String> {
    if let Value::Object(map) = v {
        map.keys().cloned().collect()
    } else {
        Vec::new()
    }
}

fn list_json_files(dir: &Path) -> Vec<PathBuf> {
    match fs::read_dir(dir) {
        Ok(read) => {
            let mut entries: Vec<_> = read.flatten().map(|e| e.path()).collect();
            entries.retain(|p| p.is_file() && p.extension() == Some("json".as_ref()));
            entries.sort();
            entries
        }
        Err(_) => Vec::new(),
    }
}

fn find_duplicates_in_file(path: &Path) -> Result<HashMap<String, usize>, String> {
    let v = read_json(path)?;
    if let Value::Object(map) = v {
        let mut counts = HashMap::new();
        for k in map.keys() {
            *counts.entry(k.clone()).or_insert(0usize) += 1;
        }
        Ok(counts.into_iter().filter(|(_, c)| *c > 1).collect())
    } else {
        Err(format!("{}: root is not an object", path.display()))
    }
}

fn write_sorted(path: &Path, base_keys: &[String]) -> Result<(), String> {
    let v = read_json(path)?;
    if let Value::Object(mut map) = v {
        let mut out: IndexMap<String, Value> = IndexMap::new();
        let mut missing = Vec::new();
        for k in base_keys {
            if let Some(val) = map.remove(k) {
                out.insert(k.clone(), val);
            } else {
                missing.push(k.clone());
            }
        }
        let mut remaining: Vec<_> = map.into_iter().collect();
        remaining.sort_by(|a, b| a.0.cmp(&b.0));
        for (k, v) in remaining {
            out.insert(k, v);
        }
        let s = serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?;
        fs::write(path, s).map_err(|e| format!("write {}: {}", path.display(), e))?;
        Ok(())
    } else {
        Err(format!("{}: root is not an object", path.display()))
    }
}

fn main() {
    let mut cmd = Command::new("cvr-i18n")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("directory")
                .short('d')
                .long("directory")
                .value_parser(clap::builder::ValueParser::os_string()),
        )
        .arg(
            Arg::new("duplicated_key")
                .short('k')
                .long("duplicated-key")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("missing_key")
                .short('m')
                .long("missing-key")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("export")
                .short('e')
                .long("export")
                .value_name("DIR"),
        )
        .arg(
            Arg::new("sort")
                .short('s')
                .long("sort")
                .action(ArgAction::SetTrue),
        )
        .arg(Arg::new("base").short('b').long("base").value_name("FILE"))
        .arg(Arg::new("file").short('f').long("file").value_name("FILE"));

    let matches = cmd.clone().get_matches();

    let dir: PathBuf = if let Some(d) = matches.get_one::<OsString>("directory") {
        d.clone().into()
    } else if Path::new("locales").exists() {
        "locales".into()
    } else if Path::new("src/locales").exists() {
        "src/locales".into()
    } else {
        eprintln!(
            "No default directory found (checked ./locales and ./src/locales). Please specify with -d"
        );
        std::process::exit(2);
    };

    let dir = dir.as_path();

    if matches.get_flag("duplicated_key") {
        if let Some(f) = matches.get_one::<String>("file") {
            let p = Path::new(f);
            match find_duplicates_in_file(p) {
                Ok(d) if d.is_empty() => println!("{}: OK", p.display()),
                Ok(d) => {
                    println!("{}: DUPLICATES:", p.display());
                    for (k, c) in d {
                        println!("  {}  ({} times)", k, c);
                    }
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("{}: ERROR: {}", p.display(), e);
                    std::process::exit(2);
                }
            }
            return;
        }
        if !dir.exists() {
            eprintln!("Directory does not exist: {}", dir.display());
            std::process::exit(2);
        }
        let mut any_errors = false;
        let mut any_duplicates = false;
        for p in list_json_files(dir) {
            match find_duplicates_in_file(&p) {
                Ok(d) if d.is_empty() => println!("{}: OK", p.display()),
                Ok(d) => {
                    any_duplicates = true;
                    println!("{}: DUPLICATES:", p.display());
                    for (k, c) in d {
                        println!("  {}  ({} times)", k, c);
                    }
                }
                Err(e) => {
                    any_errors = true;
                    eprintln!("{}: ERROR: {}", p.display(), e);
                }
            }
        }
        if any_errors {
            std::process::exit(2);
        }
        if any_duplicates {
            std::process::exit(1);
        }
        return;
    }

    if matches.get_flag("missing_key") {
        let base_file = matches
            .get_one::<String>("base")
            .map(|s| s.as_str())
            .unwrap_or("en.json");
        let base_path = if base_file.contains('/') || base_file.contains('\\') {
            Path::new(base_file).to_path_buf()
        } else {
            dir.join(base_file)
        };
        if !base_path.exists() {
            eprintln!("Base file {} not found", base_path.display());
            std::process::exit(2);
        }
        let base_v = match read_json(&base_path) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Failed to read {}: {}", base_path.display(), e);
                std::process::exit(2);
            }
        };
        let base_keys = keys_from_value(&base_v);
        let export_dir = matches.get_one::<String>("export");
        if let Some(f) = matches.get_one::<String>("file") {
            let p = Path::new(f);
            match read_json(p) {
                Ok(v) => {
                    let keys: HashSet<String> = keys_from_value(&v).into_iter().collect();
                    let missing: Vec<String> = base_keys
                        .iter()
                        .filter(|k| !keys.contains(*k))
                        .cloned()
                        .collect();
                    if missing.is_empty() {
                        println!("{}: OK", p.display());
                    } else {
                        println!("{}: MISSING:", p.display());
                        for k in &missing {
                            println!("  {}", k);
                        }
                        if let Some(ed) = export_dir {
                            let file_name = format!(
                                "{}_missing.json",
                                p.file_stem().unwrap().to_str().unwrap()
                            );
                            let export_path = Path::new(ed).join(file_name);
                            let json = serde_json::to_string_pretty(&missing).unwrap();
                            if let Err(e) = fs::write(&export_path, json) {
                                eprintln!("Failed to write {}: {}", export_path.display(), e);
                            } else {
                                println!("Exported missing keys to {}", export_path.display());
                            }
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{}: ERROR: {}", p.display(), e);
                    std::process::exit(2);
                }
            }
            return;
        }
        if !dir.exists() {
            eprintln!("Directory does not exist: {}", dir.display());
            std::process::exit(2);
        }
        let mut any_missing = false;
        for p in list_json_files(dir) {
            if p == base_path {
                continue;
            }
            match read_json(&p) {
                Ok(v) => {
                    let keys: HashSet<String> = keys_from_value(&v).into_iter().collect();
                    let missing: Vec<String> = base_keys
                        .iter()
                        .filter(|k| !keys.contains(*k))
                        .cloned()
                        .collect();
                    if missing.is_empty() {
                        println!("{}: OK", p.display());
                    } else {
                        any_missing = true;
                        println!("{}: MISSING:", p.display());
                        for k in &missing {
                            println!("  {}", k);
                        }
                        if let Some(ed) = export_dir {
                            let file_name = format!(
                                "{}_missing.json",
                                p.file_stem().unwrap().to_str().unwrap()
                            );
                            let export_path = Path::new(ed).join(file_name);
                            let json = serde_json::to_string_pretty(&missing).unwrap();
                            if let Err(e) = fs::write(&export_path, json) {
                                eprintln!("Failed to write {}: {}", export_path.display(), e);
                            } else {
                                println!("Exported missing keys to {}", export_path.display());
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{}: ERROR: {}", p.display(), e);
                }
            }
        }
        if any_missing {
            std::process::exit(1);
        } else {
            std::process::exit(0);
        }
    }

    if matches.get_flag("sort") {
        let base_file = matches
            .get_one::<String>("base")
            .map(|s| s.as_str())
            .unwrap_or("en.json");
        let base_path = if base_file.contains('/') || base_file.contains('\\') {
            Path::new(base_file).to_path_buf()
        } else {
            dir.join(base_file)
        };
        if !base_path.exists() {
            eprintln!("Base file {} not found", base_path.display());
            std::process::exit(2);
        }
        let base_indexmap: IndexMap<String, Value> =
            serde_json::from_str(&fs::read_to_string(&base_path).unwrap()).unwrap_or_else(|e| {
                eprintln!("Failed to parse {} as IndexMap: {}", base_path.display(), e);
                std::process::exit(2);
            });
        let keys: Vec<String> = base_indexmap.keys().cloned().collect();
        if let Some(f) = matches.get_one::<String>("file") {
            let p = Path::new(f);
            match write_sorted(p, &keys) {
                Ok(_) => println!("Sorted {}", p.display()),
                Err(e) => {
                    eprintln!("Failed to sort {}: {}", p.display(), e);
                    std::process::exit(2);
                }
            }
            return;
        }
        if !dir.exists() {
            eprintln!("Directory does not exist: {}", dir.display());
            std::process::exit(2);
        }
        for p in list_json_files(dir) {
            if p == base_path {
                continue;
            }
            match write_sorted(&p, &keys) {
                Ok(_) => println!("Sorted {}", p.display()),
                Err(e) => eprintln!("Failed to sort {}: {}", p.display(), e),
            }
        }
        std::process::exit(0);
    }

    println!("{}", cmd.render_help());
    std::process::exit(0);
}
