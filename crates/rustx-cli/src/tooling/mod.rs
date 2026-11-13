pub mod format;
pub mod symbols;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime},
};
pub const FRAMEWORK: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
pub fn hash(bytes: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(bytes.as_ref()))
}
pub fn run(program: &str, args: &[&str], cwd: &Path) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .status()
        .map_err(|e| format!("{program}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} exited with {status}"))
    }
}
pub fn output(program: &str, args: &[&str], cwd: &Path) -> Result<String, String> {
    let out = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("{program}: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into());
    }
    String::from_utf8(out.stdout).map_err(|e| e.to_string())
}
pub fn write(path: &Path, text: impl AsRef<[u8]>) -> Result<(), String> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    fs::write(path, text).map_err(|e| e.to_string())
}
pub fn files(root: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for e in fs::read_dir(root).map_err(|e| e.to_string())? {
        let e = e.map_err(|e| e.to_string())?;
        let p = e.path();
        if e.file_type().map_err(|e| e.to_string())?.is_symlink() {
            continue;
        }
        if p.is_dir() {
            if ![
                "target",
                "node_modules",
                "vendor",
                "dist",
                "dist-dev",
                "dist-release",
                "dist-native",
                "dist-bench",
                "releases",
                ".git",
                ".rustx",
            ]
            .contains(&e.file_name().to_str().unwrap_or(""))
                && !p.ends_with("crates/rustx-cli/assets")
            {
                files(&p, out)?;
            }
        } else {
            out.push(p)
        }
    }
    Ok(())
}
fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(from).map_err(|e| e.to_string())? {
        let e = entry.map_err(|e| e.to_string())?;
        if e.file_type().map_err(|e| e.to_string())?.is_symlink() {
            return Err("Framework source must not contain symlinks".into());
        }
        if e.path().is_dir() {
            copy_tree(&e.path(), &to.join(e.file_name()))?
        } else {
            fs::copy(e.path(), to.join(e.file_name())).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
pub fn scaffold(destination: &Path, framework: &Path) -> Result<(), String> {
    if destination.exists() {
        return Err("Destination exists; choose a new directory".into());
    }
    let framework = framework
        .canonicalize()
        .map_err(|e| format!("Framework source: {e}"))?;
    let name = destination
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("Invalid project name")?;
    if name.is_empty()
        || !name.as_bytes()[0].is_ascii_alphabetic()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("Use a Rust package name: letters, numbers, hyphens or underscores".into());
    }
    fs::create_dir_all(destination).map_err(|e| e.to_string())?;
    copy_tree(
        &framework.join("crates"),
        &destination.join("vendor/crates"),
    )?;
    copy_tree(&framework.join("web"), &destination.join("vendor/web"))?;
    fs::copy(
        framework.join("rust-toolchain.toml"),
        destination.join("vendor/rust-toolchain.toml"),
    )
    .map_err(|e| e.to_string())?;
    for file in ["LICENSE", "NOTICE.md"] {
        fs::copy(framework.join(file), destination.join("vendor").join(file))
            .map_err(|e| e.to_string())?;
    }
    write(
        &destination.join("vendor/Cargo.toml"),
        include_str!("../../templates/sdk-workspace.toml"),
    )?;
    write(
        &destination.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"
[workspace]
[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
rust-version = "1.90"
[lib]
crate-type = ["cdylib", "rlib"]
[dependencies]
rustx = {{ path = "vendor/crates/rustx" }}
rustx-engine = {{ path = "vendor/crates/rustx-engine" }}
rustx-web = {{ path = "vendor/crates/rustx-web" }}
serde_json = "1"
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "=0.2.100"
wasm-bindgen-futures = "=0.4.50"
[profile.release]
opt-level = "s"
lto = "thin"
codegen-units = 1
"#
        ),
    )?;
    write(
        &destination.join("src/app.rustx"),
        include_str!("../../templates/counter.rustx"),
    )?;
    write(
        &destination.join("src/lib.rs"),
        include_str!("../../templates/scaffold-lib.rs"),
    )?;
    write(&destination.join("src/main.rs"),format!("#[cfg(not(target_arch=\"wasm32\"))]\nfn main() {{ {}::native(); }}\n#[cfg(target_arch=\"wasm32\")]\nfn main() {{}}\n",name.replace('-',"_")))?;
    for file in ["src/lib.rs", "src/main.rs", "src/app.rustx"] {
        let path = destination.join(file);
        write(
            &path,
            format::source(&fs::read_to_string(&path).map_err(|e| e.to_string())?)?,
        )?;
    }
    write(
        &destination.join("rust-toolchain.toml"),
        include_str!(concat!(env!("OUT_DIR"), "/rust-toolchain.toml")),
    )?;
    write(
        &destination.join(".gitignore"),
        "/target/\n/dist/\n/.rustx/\n",
    )?;
    write(&destination.join(".vscode/settings.json"),"{\"files.associations\":{\"*.rustx\":\"rust\"},\"editor.formatOnSave\":true,\"rust-analyzer.check.command\":\"check\"}\n")?;
    write(&destination.join("README.md"),format!("# {name}\n\nCreated from complete local RustX source. Edit `src/app.rustx`; setup runs once per mount.\n\n```sh\nrustx doctor\nrustx dev --target web\nrustx fmt --check\nrustx check\nrustx test\nrustx build --target web --release\n```\n\nNo Node/Python/Docker runtime is required. Rust 1.90.0, the wasm32 target and wasm-bindgen 0.2.100 are required for web builds. Native Windows needs MSVC/C++ build tools. `rustx build --target windows` must run on Windows. Vendored framework source and licenses make this project independent of the original checkout. Install the supplied VS Code extension for markup formatting/completions; rust-analyzer supplies Rust diagnostics/navigation.\n"))?;
    fs::copy(framework.join("Cargo.lock"), destination.join("Cargo.lock"))
        .map_err(|e| e.to_string())?;
    let dest = destination.canonicalize().map_err(|e| e.to_string())?;
    if output("cargo", &["generate-lockfile", "--offline"], &dest).is_err() {
        run("cargo", &["generate-lockfile"], &dest)?;
    }
    println!(
        "Created {}\nNext: cd {} && rustx dev --target web",
        dest.display(),
        destination.display()
    );
    Ok(())
}
pub fn doctor(root: &Path, json_output: bool) -> Result<(), String> {
    let rust = output("rustc", &["--version"], root).unwrap_or_default();
    let binding = output("wasm-bindgen", &["--version"], root).unwrap_or_default();
    let targets = output("rustup", &["target", "list", "--installed"], root).unwrap_or_default();
    let fmt = output("rustfmt", &["--version"], root).unwrap_or_default();
    let analyzer = output("rust-analyzer", &["--version"], root).unwrap_or_default();
    let result = json!({"rust":rust.trim(),"rust_pinned":rust.starts_with("rustc 1.90.0"),"wasm_target":targets.lines().any(|s|s=="wasm32-unknown-unknown"),"wasm_bindgen":binding.trim(),"bindings_match":binding.trim()=="wasm-bindgen 0.2.100","rustfmt":fmt.trim(),"rust_analyzer":analyzer.trim(),"platform":std::env::consts::OS,"windows_build_runner":cfg!(windows),"compiler_sandbox":if cfg!(target_os="macos"){Path::new("/usr/bin/sandbox-exec").is_file()}else{false},"notes":["Tool availability is not platform/IME/screen-reader acceptance","Install missing Rust target: rustup target add wasm32-unknown-unknown","Install matching bindings: cargo install wasm-bindgen-cli --version 0.2.100 --locked"]});
    if json_output {
        println!("{}", serde_json::to_string_pretty(&result).unwrap());
    } else {
        for (key, value) in result.as_object().unwrap() {
            println!("{key}: {value}");
        }
    }
    if result["rust_pinned"] != true
        || result["wasm_target"] != true
        || result["bindings_match"] != true
        || fmt.is_empty()
    {
        Err("Doctor found missing/mismatched prerequisites".into())
    } else {
        Ok(())
    }
}
pub fn format_project(root: &Path, check: bool, stdin: bool) -> Result<(), String> {
    if stdin {
        let mut input = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut input)
            .map_err(|e| e.to_string())?;
        print!("{}", format::source(&input)?);
        return Ok(());
    }
    let mut paths = vec![];
    files(root, &mut paths)?;
    let mut changes = vec![];
    for p in paths {
        if p.components()
            .map(|p| p.as_os_str())
            .collect::<Vec<_>>()
            .windows(2)
            .any(|w| w[0] == "tests" && w[1] == "ui")
        {
            continue;
        }
        if matches!(p.extension().and_then(|s| s.to_str()), Some("rs" | "rustx")) {
            let input = fs::read_to_string(&p).map_err(|e| e.to_string())?;
            let formatted = format::source(&input).map_err(|e| format!("{}: {e}", p.display()))?;
            if input != formatted {
                changes.push(p.clone());
                if !check {
                    write(&p, formatted)?;
                }
            }
        }
    }
    for p in &changes {
        eprintln!(
            "{} {}",
            if check { "Would format" } else { "Formatted" },
            p.display()
        );
    }
    if check && !changes.is_empty() {
        Err("Formatting changes required".into())
    } else {
        Ok(())
    }
}
fn package(root: &Path) -> Result<(String, String, PathBuf), String> {
    let v: Value = serde_json::from_str(&output(
        "cargo",
        &["metadata", "--no-deps", "--format-version", "1", "--locked"],
        root,
    )?)
    .map_err(|e| e.to_string())?;
    let packages = v["packages"].as_array().ok_or("No packages")?;
    let p = packages
        .iter()
        .find(|p| Path::new(p["manifest_path"].as_str().unwrap()).parent() == Some(root))
        .or_else(|| packages.iter().find(|p| p["name"] == "rustx-lab"))
        .ok_or("Run from an application package or framework root")?;
    let name = p["name"].as_str().unwrap().to_owned();
    let module = p["targets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| {
            t["crate_types"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == "cdylib")
        })
        .ok_or("Application needs [lib] crate-type=[\"cdylib\",\"rlib\"]")?["name"]
        .as_str()
        .unwrap()
        .to_owned();
    Ok((
        name,
        module,
        PathBuf::from(v["target_directory"].as_str().unwrap()),
    ))
}
const HOST: &str = include_str!(concat!(env!("OUT_DIR"), "/host.js"));
const INDEX: &str = include_str!(concat!(env!("OUT_DIR"), "/index.html"));
const SERVICES: &str = include_str!(concat!(env!("OUT_DIR"), "/services.js"));
const SEMANTICS: &str = include_str!(concat!(env!("OUT_DIR"), "/semantics.js"));
pub fn build(
    root: &Path,
    target: &str,
    out: &Path,
    devtools: bool,
    compiler: Option<&str>,
    instrument: bool,
    benchmark: bool,
) -> Result<(), String> {
    if out == root
        || !out.starts_with(root)
        || out
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
        || out
            .ancestors()
            .any(|p| fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink()))
    {
        return Err("Output must be a real child directory of the project".into());
    }
    if out.exists() && !out.join("assets.json").exists() && !out.join("pkg").exists() {
        return Err("Refusing to replace a directory without a RustX bundle manifest".into());
    }
    if let Some(origin) = compiler {
        let host = origin
            .strip_prefix("http://")
            .or_else(|| origin.strip_prefix("https://"))
            .ok_or("Compiler URL must be an HTTP(S) origin")?;
        if host.is_empty()
            || !host
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".:-".contains(&b))
        {
            return Err("Compiler URL must be an origin without a path or credentials".into());
        }
    }
    if target != "web" {
        if target == "windows" && !cfg!(windows) {
            return Err("Windows packaging requires an actual Windows/MSVC runner; no cross-build is presented as execution evidence".into());
        }
        if !["native", "windows"].contains(&target) {
            return Err("Target must be web, native or windows".into());
        }
        run("cargo", &["build", "--release", "--locked"], root)?;
        let (name, _, dir) = package(root)?;
        let suffix = if cfg!(windows) { ".exe" } else { "" };
        fs::create_dir_all(out).map_err(|e| e.to_string())?;
        fs::copy(
            dir.join("release").join(format!("{name}{suffix}")),
            out.join(format!("{name}{suffix}")),
        )
        .map_err(|e| e.to_string())?;
        write(
            &out.join("assets.json"),
            b"{\"version\":1,\"target\":\"native\"}",
        )?;
        write(&out.join("RUN.md"),"Run the included executable. Native execution/IME/accessibility/DPI acceptance must be recorded on this actual target.\n")?;
        return Ok(());
    }
    let (name, module, target_dir) = package(root)?;
    let mut args = vec![
        "build",
        "-p",
        &name,
        "--lib",
        "--target",
        "wasm32-unknown-unknown",
        "--release",
        "--locked",
    ];
    if instrument {
        args.extend(["--features", "instrumentation"]);
    }
    let built = Command::new("cargo")
        .args(&args)
        .current_dir(root)
        .output()
        .map_err(|e| e.to_string())?;
    if !built.status.success() {
        return Err(String::from_utf8_lossy(&built.stderr)
            .chars()
            .take(65536)
            .collect());
    }
    eprint!("{}", String::from_utf8_lossy(&built.stderr));
    let temporary = root.join(".rustx/bundle-next");
    if temporary.exists() {
        fs::remove_dir_all(&temporary).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(temporary.join("pkg")).map_err(|e| e.to_string())?;
    let wasm = target_dir
        .join("wasm32-unknown-unknown/release")
        .join(format!("{module}.wasm"));
    run(
        "wasm-bindgen",
        &[
            wasm.to_str().ok_or("Invalid output path")?,
            "--target",
            "web",
            "--out-dir",
            temporary.join("pkg").to_str().unwrap(),
        ],
        root,
    )?;
    for (name, bytes) in [
        (
            "host.js",
            HOST.replace("./pkg/rustx_lab.js", &format!("./pkg/{module}.js")),
        ),
        ("index.html", INDEX.into()),
        ("services.js", SERVICES.into()),
        ("semantics.js", SEMANTICS.into()),
    ] {
        write(&temporary.join(name), bytes)?;
    }
    let config = json!({"version":1,"devtools":devtools,"instrumentation":instrument,"benchmark":benchmark,"compiler":compiler,"source_root":if devtools{Some(root.to_string_lossy())}else{None}});
    write(
        &temporary.join("runtime-config.json"),
        serde_json::to_vec(&config).unwrap(),
    )?;
    if compiler.is_some() {
        write(
            &temporary.join("playground.js"),
            include_str!(concat!(env!("OUT_DIR"), "/playground.js")),
        )?;
    }
    if devtools {
        for (name, source) in [
            (
                "devtools.js",
                include_str!(concat!(env!("OUT_DIR"), "/devtools.js")),
            ),
            (
                "inspector.html",
                include_str!(concat!(env!("OUT_DIR"), "/inspector.html")),
            ),
            (
                "inspector.js",
                include_str!(concat!(env!("OUT_DIR"), "/inspector.js")),
            ),
            (
                "inspector.css",
                include_str!(concat!(env!("OUT_DIR"), "/inspector.css")),
            ),
        ] {
            write(&temporary.join(name), source)?;
        }
    }
    let guide = include_str!(concat!(env!("OUT_DIR"), "/tutorial.md"))
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    write(
        &temporary.join("docs/index.html"),
        include_str!("../../templates/documentation.html").replace("__RUSTX_GUIDE__", &guide),
    )?;
    let digest = |text: &[u8]| hash(text)[..12].to_owned();
    let raw_wasm =
        fs::read(temporary.join(format!("pkg/{module}_bg.wasm"))).map_err(|e| e.to_string())?;
    let wasm_name = format!("{module}_bg.{}.wasm", digest(&raw_wasm));
    write(&temporary.join("pkg").join(&wasm_name), &raw_wasm)?;
    let js = fs::read_to_string(temporary.join(format!("pkg/{module}.js")))
        .map_err(|e| e.to_string())?
        .replace(&format!("{module}_bg.wasm"), &wasm_name);
    let js_name = format!("{module}.{}.js", digest(js.as_bytes()));
    write(&temporary.join("pkg").join(&js_name), js)?;
    let services_name = format!("services.{}.js", digest(SERVICES.as_bytes()));
    let semantics_name = format!("semantics.{}.js", digest(SEMANTICS.as_bytes()));
    write(&temporary.join(&services_name), SERVICES)?;
    write(&temporary.join(&semantics_name), SEMANTICS)?;
    let host = HOST
        .replace("./pkg/rustx_lab.js", &format!("./pkg/{js_name}"))
        .replace("./services.js", &format!("./{services_name}"))
        .replace("./semantics.js", &format!("./{semantics_name}"));
    let host_name = format!("host.{}.js", digest(host.as_bytes()));
    write(&temporary.join(&host_name), host)?;
    write(
        &temporary.join("index.html"),
        INDEX
            .replace("/host.js", &format!("/{host_name}"))
            .replace("<!-- RUSTX_WASM_PRELOAD -->", &format!(
                r#"<link rel="preload" href="/pkg/{wasm_name}" as="fetch" type="application/wasm" crossorigin />"#,
            )),
    )?;
    let mut paths = vec![];
    files(&temporary, &mut paths)?;
    let mut manifest = serde_json::Map::new();
    for path in &paths {
        manifest.insert(
            path.strip_prefix(&temporary)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/"),
            json!(hash(fs::read(path).map_err(|e| e.to_string())?)),
        );
    }
    let build_id = hash(serde_json::to_vec(&manifest).unwrap());
    let precache = manifest
        .keys()
        .filter(|p| {
            p.as_str() == "index.html"
                || p.as_str() == "runtime-config.json"
                || p.as_str() == "playground.js"
                || p.split('.')
                    .any(|part| part.len() == 12 && part.bytes().all(|b| b.is_ascii_hexdigit()))
                || devtools
                    && [
                        "devtools.js",
                        "inspector.html",
                        "inspector.css",
                        "inspector.js",
                    ]
                    .contains(&p.as_str())
        })
        .cloned()
        .collect::<Vec<_>>();
    let urls = precache.iter().map(|s| format!("/{s}")).collect::<Vec<_>>();
    let worker = include_str!(concat!(env!("OUT_DIR"), "/service-worker.js"))
        .replace("__RUSTX_CACHE__", &format!("rustx-{}", &build_id[..16]))
        .replace("__RUSTX_FILES__", &serde_json::to_string(&urls).unwrap());
    write(&temporary.join("service-worker.js"), worker)?;
    write(&temporary.join("assets.json"),serde_json::to_vec_pretty(&json!({"version":1,"build":build_id,"module":module,"devtools":devtools,"files":manifest})).unwrap())?;
    let mut compress = vec![];
    files(&temporary, &mut compress)?;
    for path in compress {
        if matches!(
            path.extension().and_then(|s| s.to_str()),
            Some("js" | "wasm" | "html" | "json" | "css")
        ) {
            let data = fs::read(&path).map_err(|e| e.to_string())?;
            let mut encoder =
                flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::new(6));
            std::io::Write::write_all(&mut encoder, &data).map_err(|e| e.to_string())?;
            write(
                &PathBuf::from(format!("{}.gz", path.display())),
                encoder.finish().map_err(|e| e.to_string())?,
            )?;
            let mut compressed = Vec::new();
            {
                let mut encoder = brotli::CompressorWriter::new(&mut compressed, 65536, 9, 22);
                std::io::Write::write_all(&mut encoder, &data).map_err(|e| e.to_string())?;
            }
            write(&PathBuf::from(format!("{}.br", path.display())), compressed)?;
        }
    }
    if out.exists() {
        fs::remove_dir_all(out).map_err(|e| e.to_string())?;
    }
    fs::rename(&temporary, out).map_err(|e| e.to_string())?;
    println!("Built {} ({build_id})", out.display());
    Ok(())
}
pub fn dev(root: &Path, target: &str, port: &str, compiler: Option<&str>) -> Result<(), String> {
    let out = root.join(if target == "web" {
        "dist"
    } else {
        "dist-native"
    });
    build(root, target, &out, true, compiler, false, false)?;
    let fingerprint = || -> Result<String, String> {
        let mut paths = vec![];
        files(root, &mut paths)?;
        paths.sort();
        let mut h = Sha256::new();
        for p in paths {
            if matches!(
                p.extension().and_then(|s| s.to_str()),
                Some("rs" | "rustx" | "toml" | "js" | "html")
            ) {
                h.update(fs::read(p).map_err(|e| e.to_string())?);
            }
        }
        Ok(format!("{:x}", h.finalize()))
    };
    let mut previous = fingerprint()?;
    if target != "web" {
        let (name, _, dir) = package(root)?;
        let executable = dir
            .join("release")
            .join(format!("{name}{}", if cfg!(windows) { ".exe" } else { "" }));
        let mut child = Command::new(&executable)
            .current_dir(root)
            .spawn()
            .map_err(|e| e.to_string())?;
        loop {
            std::thread::sleep(Duration::from_millis(500));
            if child.try_wait().map_err(|e| e.to_string())?.is_some() {
                return Ok(());
            }
            let next = fingerprint()?;
            if next != previous {
                previous = next;
                match build(root, target, &out, true, compiler, false, false) {
                    Ok(()) => {
                        let _ = child.kill();
                        let _ = child.wait();
                        child = Command::new(&executable)
                            .current_dir(root)
                            .spawn()
                            .map_err(|e| e.to_string())?;
                    }
                    Err(e) => eprintln!("Build failed; fix the source to rebuild:\n{e}"),
                }
            }
        }
    }
    write(&out.join("dev.json"), b"{\"revision\":0,\"error\":null}")?;
    let served = out.clone();
    let port = port.to_owned();
    std::thread::spawn(move || {
        if let Err(e) = crate::server::serve(&served, &port) {
            eprintln!("{e}");
        }
    });
    loop {
        std::thread::sleep(Duration::from_millis(500));
        let next = fingerprint()?;
        if next != previous {
            previous = next;
            let result = build(root, "web", &out, true, compiler, false, false);
            let state = match result {
                Ok(()) => {
                    json!({"revision":SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis(),"error":null})
                }
                Err(e) => json!({"error":e}),
            };
            write(&out.join("dev.json"), serde_json::to_vec(&state).unwrap())?;
        }
    }
}

/// The installed CLI carries its SDK; project creation does not depend on a build-machine path.
pub fn framework(root: &Path, provided: Option<&str>) -> Result<PathBuf, String> {
    if let Some(path) = provided {
        return Path::new(path).canonicalize().map_err(|e| e.to_string());
    }
    const BUNDLE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/framework.bin"));
    let identity = hash(BUNDLE);
    let sdk = root.join(".rustx").join(format!("sdk-{}", &identity[..16]));
    if sdk.join("SDK-IDENTITY.txt").is_file() {
        return Ok(sdk);
    }
    let mut random = [0; 8];
    getrandom::fill(&mut random).map_err(|e| e.to_string())?;
    let next = root
        .join(".rustx")
        .join(format!("sdk-extract-{}", hash(random)));
    let mut offset = 0;
    while offset < BUNDLE.len() {
        if offset + 12 > BUNDLE.len() {
            return Err("Invalid embedded SDK".into());
        }
        let name_len = u32::from_le_bytes(BUNDLE[offset..offset + 4].try_into().unwrap()) as usize;
        let bytes =
            u64::from_le_bytes(BUNDLE[offset + 4..offset + 12].try_into().unwrap()) as usize;
        offset += 12;
        if name_len > 4096 || bytes > 32 * 1024 * 1024 || offset + name_len + bytes > BUNDLE.len() {
            return Err("Invalid embedded SDK bounds".into());
        }
        let name = std::str::from_utf8(&BUNDLE[offset..offset + name_len])
            .map_err(|_| "Invalid SDK path")?;
        if Path::new(name).is_absolute()
            || Path::new(name)
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err("Invalid SDK path".into());
        }
        offset += name_len;
        write(&next.join(name), &BUNDLE[offset..offset + bytes])?;
        offset += bytes;
    }
    write(&next.join("SDK-IDENTITY.txt"), &identity)?;
    match fs::rename(&next, &sdk) {
        Ok(()) => {}
        Err(_) if sdk.join("SDK-IDENTITY.txt").is_file() => {
            let _ = fs::remove_dir_all(&next);
        }
        Err(e) => return Err(e.to_string()),
    }
    Ok(sdk)
}
