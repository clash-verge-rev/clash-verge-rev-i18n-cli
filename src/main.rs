use clap::{Arg, ArgAction, Command};
use indexmap::IndexMap;
use serde_json::Value;
use std::collections::HashMap;
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

fn extract_top_level_keys(s: &str) -> Vec<String> {
    match serde_json::from_str::<Value>(s) {
        Ok(Value::Object(map)) => map.keys().cloned().collect(),
        _ => Vec::new(),
    }
}

fn find_duplicates_in_file(path: &Path) -> Result<HashMap<String, usize>, String> {
    let s = fs::read_to_string(path).map_err(|e| format!("read {}: {}", path.display(), e))?;
    let keys = extract_top_level_keys(&s);
    let mut counts = HashMap::new();
    for k in keys {
        *counts.entry(k).or_insert(0usize) += 1;
    }
    let dups: HashMap<String, usize> = counts.into_iter().filter(|(_, v)| *v > 1).collect();
    if dups.is_empty() {
        // validate JSON syntax
        let _: Value = serde_json::from_str(&s).map_err(|e| format!("invalid JSON: {}", e))?;
    }
    Ok(dups)
}

fn main() {
    let mut cmd = Command::new("cvr-i18n")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("directory")
                .short('d')
                .long("directory")
                .value_parser(clap::builder::ValueParser::os_string())
                .help("Directory to use, default is ./locales and ./src/locales"),
        )
        .arg(
            Arg::new("duplicated_key")
                .short('k')
                .long("duplicated-key")
                .help("Check for duplicate top-level keys in each JSON file")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("missing_key")
                .short('m')
                .long("missing-key")
                .help("Check for missing top-level keys in each JSON file compared to en.json")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("export")
                .short('e')
                .long("export")
                .value_name("DIR")
                .help("Export missing keys to JSON files in the specified directory"),
        )
        .arg(
            Arg::new("sort")
                .short('s')
                .long("sort")
                .help("Sort keys in JSON files according to the base file's key order, the default is en.json")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("base")
                .short('b')
                .long("base")
                .value_name("FILE")
                .help("Base file for key order, default is en.json"),
        )
        .arg(
            Arg::new("file")
                .short('f')
                .long("file")
                .value_name("FILE")
                .help("Specify a single file to process instead of the entire directory"),
        );

    let matches = cmd.clone().get_matches();

    let dir: PathBuf = if let Some(d) = matches.get_one::<OsString>("directory") {
        d.into()
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
        if let Some(file) = matches.get_one::<String>("file") {
            let path = Path::new(file);
            match find_duplicates_in_file(path) {
                Ok(dups) => {
                    if dups.is_empty() {
                        println!("{}: OK", path.display());
                    } else {
                        println!("{}: DUPLICATES:", path.display());
                        for (k, c) in dups {
                            println!("  {}  ({} times)", k, c);
                        }
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("{}: ERROR: {}", path.display(), e);
                    std::process::exit(2);
                }
            }
            std::process::exit(0);
        } else {
            if !dir.exists() {
                eprintln!("Directory does not exist: {}", dir.display());
                std::process::exit(2);
            }

            let mut any_errors = false;
            let mut any_duplicates = false;

            let read = fs::read_dir(dir).unwrap_or_else(|e| {
                eprintln!("Failed to read directory {}: {}", dir.display(), e);
                std::process::exit(2);
            });

            for entry in read.flatten() {
                let path = entry.path();
                if path.is_file()
                    && let Some(ext) = path.extension()
                    && ext == "json"
                {
                    match find_duplicates_in_file(&path) {
                        Ok(dups) => {
                            if dups.is_empty() {
                                println!("{}: OK", path.display());
                            } else {
                                any_duplicates = true;
                                println!("{}: DUPLICATES:", path.display());
                                for (k, c) in dups {
                                    println!("  {}  ({} times)", k, c);
                                }
                            }
                        }
                        Err(e) => {
                            any_errors = true;
                            eprintln!("{}: ERROR: {}", path.display(), e);
                        }
                    }
                }
            }

            if any_errors {
                std::process::exit(2);
            }
            if any_duplicates {
                std::process::exit(1);
            }
            std::process::exit(0);
        }
    }

    if matches.get_flag("missing_key") {
        let base_file = matches
            .get_one::<String>("base")
            .map(|s| s.as_str())
            .unwrap_or("en.json");
        // Resolve base path: if user provided an explicit path (contains / or \\), use it.
        // Otherwise always use the `dir` (the chosen locales directory) joined with base_file.
        // This makes the default base file `en.json` come from the `-d`/default directory
        // even when a single file is passed with -f.
        let base_path = if base_file.contains('/') || base_file.contains('\\') {
            Path::new(base_file).to_path_buf()
        } else {
            dir.join(base_file)
        };
        if !base_path.exists() {
            eprintln!("Base file {} not found", base_path.display());
            std::process::exit(2);
        }

        let base_s = fs::read_to_string(&base_path).unwrap_or_else(|e| {
            eprintln!("Failed to read {}: {}", base_path.display(), e);
            std::process::exit(2);
        });

        let base_value: Value = serde_json::from_str(&base_s).unwrap_or_else(|e| {
            eprintln!("Failed to parse {}: {}", base_path.display(), e);
            std::process::exit(2);
        });

        // preserve base key order for reporting missing keys
        let base_keys_vec: Vec<String> = if let Value::Object(map) = base_value {
            map.keys().cloned().collect()
        } else {
            eprintln!("{}: root is not an object", base_path.display());
            std::process::exit(2);
        };

        let export_dir = matches.get_one::<String>("export");
        if let Some(ed) = export_dir
            && let Err(e) = fs::create_dir_all(ed)
        {
            eprintln!("Failed to create export directory {}: {}", ed, e);
            std::process::exit(2);
        }

        if let Some(file) = matches.get_one::<String>("file") {
            let path = Path::new(file);
            match fs::read_to_string(path) {
                Ok(s) => match serde_json::from_str(&s) {
                    Ok(value) => {
                        let keys: HashSet<String> = if let Value::Object(map) = value {
                            map.keys().cloned().collect()
                        } else {
                            eprintln!("{}: root is not an object", path.display());
                            std::process::exit(2);
                        };

                        // collect missing keys in the same order as base file
                        let missing: Vec<String> = base_keys_vec
                            .iter()
                            .filter(|k| !keys.contains(*k))
                            .cloned()
                            .collect();
                        if missing.is_empty() {
                            println!("{}: OK", path.display());
                        } else {
                            println!("{}: MISSING:", path.display());
                            for k in &missing {
                                println!("  {}", k);
                            }
                            if let Some(ed) = export_dir {
                                let file_name = format!(
                                    "{}_missing.json",
                                    path.file_stem().unwrap().to_str().unwrap()
                                );
                                let export_path = Path::new(ed).join(file_name);
                                let json = serde_json::to_string_pretty(&missing).unwrap();
                                if let Err(e) = fs::write(&export_path, json) {
                                    eprintln!("Failed to write {}: {}", export_path.display(), e);
                                } else {
                                    println!("Exported missing keys to {}", export_path.display());
                                }
                            }
                            std::process::exit(1);
                        }
                    }
                    Err(e) => {
                        eprintln!("{}: ERROR: parse {}", path.display(), e);
                        std::process::exit(2);
                    }
                },
                Err(e) => {
                    eprintln!("Failed to read {}: {}", path.display(), e);
                    std::process::exit(2);
                }
            }
            std::process::exit(0);
        } else {
            if !dir.exists() {
                eprintln!("Directory does not exist: {}", dir.display());
                std::process::exit(2);
            }

            let en_path = dir.join("en.json");
            if !en_path.exists() {
                eprintln!("en.json not found in {}", dir.display());
                std::process::exit(2);
            }

            let en_s = fs::read_to_string(&en_path).unwrap_or_else(|e| {
                eprintln!("Failed to read {}: {}", en_path.display(), e);
                std::process::exit(2);
            });

            let en_value: Value = serde_json::from_str(&en_s).unwrap_or_else(|e| {
                eprintln!("Failed to parse {}: {}", en_path.display(), e);
                std::process::exit(2);
            });

            // preserve en.json key order for reporting missing keys
            let en_keys_vec: Vec<String> = if let Value::Object(map) = en_value {
                map.keys().cloned().collect()
            } else {
                eprintln!("{}: root is not an object", en_path.display());
                std::process::exit(2);
            };

            let mut any_missing = false;

            let read = fs::read_dir(dir).unwrap_or_else(|e| {
                eprintln!("Failed to read directory {}: {}", dir.display(), e);
                std::process::exit(2);
            });

            for entry in read.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension() == Some("json".as_ref()) && path != en_path {
                    match fs::read_to_string(&path) {
                        Ok(s) => match serde_json::from_str(&s) {
                            Ok(value) => {
                                let keys: HashSet<String> = if let Value::Object(map) = value {
                                    map.keys().cloned().collect()
                                } else {
                                    eprintln!("{}: root is not an object", path.display());
                                    continue;
                                };

                                // collect missing keys in en.json order
                                let missing: Vec<String> = en_keys_vec
                                    .iter()
                                    .filter(|k| !keys.contains(*k))
                                    .cloned()
                                    .collect();
                                if missing.is_empty() {
                                    println!("{}: OK", path.display());
                                } else {
                                    any_missing = true;
                                    println!("{}: MISSING:", path.display());
                                    for k in &missing {
                                        println!("  {}", k);
                                    }
                                    if let Some(ed) = export_dir {
                                        let file_name = format!(
                                            "{}_missing.json",
                                            path.file_stem().unwrap().to_str().unwrap()
                                        );
                                        let export_path = Path::new(ed).join(file_name);
                                        let json = serde_json::to_string_pretty(&missing).unwrap();
                                        if let Err(e) = fs::write(&export_path, json) {
                                            eprintln!(
                                                "Failed to write {}: {}",
                                                export_path.display(),
                                                e
                                            );
                                        } else {
                                            println!(
                                                "Exported missing keys to {}",
                                                export_path.display()
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("{}: ERROR: parse {}", path.display(), e);
                            }
                        },
                        Err(e) => {
                            eprintln!("Failed to read {}: {}", path.display(), e);
                        }
                    }
                }
            }

            if any_missing {
                std::process::exit(1);
            } else {
                std::process::exit(0);
            }
        }
    }

    if matches.get_flag("sort") {
        let base_file = matches
            .get_one::<String>("base")
            .map(|s| s.as_str())
            .unwrap_or("en.json");
        // Resolve base path: prefer explicit path if provided, otherwise use `dir/en.json`
        let base_path = if base_file.contains('/') || base_file.contains('\\') {
            Path::new(base_file).to_path_buf()
        } else {
            dir.join(base_file)
        };
        if !base_path.exists() {
            eprintln!("Base file {} not found", base_path.display());
            std::process::exit(2);
        }

        let base_s = fs::read_to_string(&base_path).unwrap_or_else(|e| {
            eprintln!("Failed to read {}: {}", base_path.display(), e);
            std::process::exit(2);
        });

        let base_indexmap: IndexMap<String, Value> =
            serde_json::from_str(&base_s).unwrap_or_else(|e| {
                eprintln!("Failed to parse {} as IndexMap: {}", base_path.display(), e);
                std::process::exit(2);
            });


        // Use keys from the parsed IndexMap to preserve the textual order from the base file
        let keys: Vec<String> = base_indexmap.keys().cloned().collect();

        

        if let Some(file) = matches.get_one::<String>("file") {
            let path = Path::new(file);
            match fs::read_to_string(path) {
                Ok(s) => match serde_json::from_str(&s) {
                    Ok(value) => {
                        if let Value::Object(mut map) = value {
                            let mut sorted_map: IndexMap<String, Value> = IndexMap::new();
                            let mut missing_base: Vec<String> = Vec::new();
                            for key in &keys {
                                if let Some(v) = map.remove(key) {
                                    // if debug { eprintln!("[debug] inserting base key: {}", key); }
                                    sorted_map.insert(key.clone(), v);
                                } else {
                                    missing_base.push(key.clone());
                                }
                            }
                            
                            let mut remaining: Vec<(String, Value)> = map.into_iter().collect();
                            remaining.sort_by(|a, b| a.0.cmp(&b.0));
                            
                            for (k, v) in remaining {
                                // if debug { eprintln!("[debug] inserting remaining key: {}", k); }
                                sorted_map.insert(k, v);
                            }
                            
                            let json = serde_json::to_string_pretty(&sorted_map).unwrap();
                            if let Err(e) = fs::write(path, json) {
                                eprintln!("Failed to write {}: {}", path.display(), e);
                                std::process::exit(2);
                            } else {
                                println!("Sorted {}", path.display());
                            }
                        } else {
                            eprintln!("{}: root is not an object", path.display());
                            std::process::exit(2);
                        }
                    }
                    Err(e) => {
                        eprintln!("{}: ERROR: parse {}", path.display(), e);
                        std::process::exit(2);
                    }
                },
                Err(e) => {
                    eprintln!("Failed to read {}: {}", path.display(), e);
                    std::process::exit(2);
                }
            }
            std::process::exit(0);
        } else {
            if !dir.exists() {
                eprintln!("Directory does not exist: {}", dir.display());
                std::process::exit(2);
            }

            let read = fs::read_dir(dir).unwrap_or_else(|e| {
                eprintln!("Failed to read directory {}: {}", dir.display(), e);
                std::process::exit(2);
            });

            let mut entries: Vec<_> = read.flatten().collect();
            entries.sort_by_key(|entry| entry.path());

            for entry in entries {
                let path = entry.path();
                if path.is_file() && path.extension() == Some("json".as_ref()) && path != base_path
                {
                    match fs::read_to_string(&path) {
                        Ok(s) => match serde_json::from_str(&s) {
                            Ok(value) => {
                                if let Value::Object(mut map) = value {
                                    let mut sorted_map: IndexMap<String, Value> = IndexMap::new();
                                    let mut missing_base: Vec<String> = Vec::new();
                                    for key in &keys {
                                        if let Some(v) = map.remove(key) {
                                            // if debug { eprintln!("[debug] inserting base key: {}", key); }
                                            sorted_map.insert(key.clone(), v);
                                        } else {
                                            missing_base.push(key.clone());
                                        }
                                    }
                                    
                                    // add remaining keys
                                    let mut remaining: Vec<(String, Value)> =
                                        map.into_iter().collect();
                                    remaining.sort_by(|a, b| a.0.cmp(&b.0));
                                    
                                    for (k, v) in remaining {
                                        // if debug { eprintln!("[debug] inserting remaining key: {}", k); }
                                        sorted_map.insert(k, v);
                                    }
                                    
                                    let json = serde_json::to_string_pretty(&sorted_map).unwrap();
                                    if let Err(e) = fs::write(&path, json) {
                                        eprintln!("Failed to write {}: {}", path.display(), e);
                                    } else {
                                        println!("Sorted {}", path.display());
                                    }
                                } else {
                                    eprintln!("{}: root is not an object", path.display());
                                }
                            }
                            Err(e) => {
                                eprintln!("{}: ERROR: parse {}", path.display(), e);
                            }
                        },
                        Err(e) => {
                            eprintln!("Failed to read {}: {}", path.display(), e);
                        }
                    }
                }
            }

            std::process::exit(0);
        }
    }

    // default behavior: show help
    println!("{}", cmd.render_help());
    std::process::exit(0);
}
