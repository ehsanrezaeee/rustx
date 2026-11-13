mod playground;
mod server;
mod tooling;
use std::path::Path;
fn option<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    args.iter()
        .position(|s| s == key)
        .and_then(|n| args.get(n + 1))
        .map(String::as_str)
}
fn validate(args: &[String]) -> Result<(), String> {
    let Some(command) = args.first().map(String::as_str) else {
        return Ok(());
    };
    let (flags, values, positionals): (&[&str], &[&str], usize) = match command {
        "new" => (&[], &["--framework"], 1),
        "doctor" => (&["--json"], &[], 0),
        "fmt" => (&["--check", "--stdin"], &[], 0),
        "check" => (&["--json", "--message-format=json"], &[], 0),
        "test" | "symbols" | "version" | "--version" | "help" | "--help" | "-h" => (&[], &[], 0),
        "serve" => (&[], &[], 2),
        "playground" => (&[], &["--framework", "--port", "--app-origin"], 0),
        "build" => (
            &["--release", "--devtools", "--instrument", "--benchmark"],
            &["--target", "--out", "--compiler-url"],
            0,
        ),
        "dev" => (&[], &["--target", "--port", "--compiler-url"], 0),
        _ => return Ok(()),
    };
    let mut seen = std::collections::HashSet::new();
    let mut positions = 0;
    let mut i = 1;
    while i < args.len() {
        let arg = args[i].as_str();
        if flags.contains(&arg) || values.contains(&arg) {
            if !seen.insert(arg) {
                return Err(format!("Duplicate option {arg}"));
            }
            if values.contains(&arg) {
                i += 1;
                if args.get(i).is_none_or(|s| s.starts_with('-')) {
                    return Err(format!("{arg} requires a value"));
                }
            }
        } else if arg.starts_with('-') {
            return Err(format!("Unknown option {arg} for {command}"));
        } else {
            positions += 1;
            if positions > positionals {
                return Err(format!("Unexpected argument {arg} for {command}"));
            }
        }
        i += 1;
    }
    if command == "fmt" && seen.contains("--stdin") && seen.contains("--check") {
        return Err("Use --stdin or --check, separately".into());
    }
    Ok(())
}
fn main() {
    if let Err(e) = execute() {
        eprintln!("RustX: {e}");
        std::process::exit(1)
    }
}
fn execute() -> Result<(), String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    validate(&args)?;
    let root = std::env::current_dir()
        .map_err(|e| e.to_string())?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    match args.first().map(String::as_str) {
        Some("serve") => server::serve(
            Path::new(args.get(1).map(String::as_str).unwrap_or("dist")),
            args.get(2).map(String::as_str).unwrap_or("3000"),
        ),
        Some("new") => tooling::scaffold(
            Path::new(
                args.get(1)
                    .filter(|s| !s.starts_with('-'))
                    .ok_or("Usage: rustx new my-app [--framework PATH]")?,
            ),
            &tooling::framework(&root, option(&args, "--framework"))?,
        ),
        Some("doctor") => tooling::doctor(&root, args.iter().any(|s| s == "--json")),
        Some("symbols") => {
            println!("{}", tooling::symbols::project(&root)?);
            Ok(())
        }
        Some("playground") => playground::serve(
            &if let Some(path) = option(&args, "--framework") {
                Path::new(path).canonicalize().map_err(|e| e.to_string())?
            } else if Path::new(tooling::FRAMEWORK)
                .join("crates/rustx-core/src/lib.rs")
                .is_file()
            {
                Path::new(tooling::FRAMEWORK)
                    .canonicalize()
                    .map_err(|e| e.to_string())?
            } else {
                tooling::framework(&root, None)?
            },
            option(&args, "--port")
                .unwrap_or("3002")
                .parse()
                .map_err(|_| "Invalid port")?,
            option(&args, "--app-origin").unwrap_or("http://127.0.0.1:3000"),
        ),
        Some("fmt") => tooling::format_project(
            &root,
            args.iter().any(|s| s == "--check"),
            args.iter().any(|s| s == "--stdin"),
        ),
        Some("check") => {
            let mut a = vec!["check", "--workspace", "--locked"];
            if args
                .iter()
                .any(|s| s == "--json" || s == "--message-format=json")
            {
                a.push("--message-format=json");
            }
            tooling::run("cargo", &a, &root)
        }
        Some("test") => tooling::run("cargo", &["test", "--workspace", "--locked"], &root),
        Some("build") => tooling::build(
            &root,
            option(&args, "--target").unwrap_or("web"),
            &root.join(option(&args, "--out").unwrap_or("dist")),
            args.iter().any(|s| s == "--devtools"),
            option(&args, "--compiler-url"),
            args.iter().any(|s| s == "--instrument"),
            args.iter().any(|s| s == "--benchmark"),
        ),
        Some("dev") => tooling::dev(
            &root,
            option(&args, "--target").unwrap_or("web"),
            option(&args, "--port").unwrap_or("3000"),
            option(&args, "--compiler-url"),
        ),
        Some("--version" | "version") => {
            println!("rustx 0.1.0-stage4 (Rust 1.90.0; bindings 0.2.100)");
            Ok(())
        }
        None | Some("help" | "--help" | "-h") => {
            println!("RustX\n  rustx new my-app [--framework PATH]\n  rustx doctor [--json]\n  rustx dev --target web|native|windows [--port 3000] [--compiler-url ORIGIN]\n  rustx check [--json]\n  rustx fmt [--check|--stdin]\n  rustx test\n  rustx build --target web|native|windows --release [--out dist] [--devtools]\n  rustx playground [--port 3002] [--app-origin http://127.0.0.1:3000]\n  rustx serve dist 3000");
            Ok(())
        }
        Some(command) => Err(format!("Unknown command {command}; use rustx --help")),
    }
}
