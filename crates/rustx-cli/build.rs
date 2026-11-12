use std::{
    env, fs,
    path::{Path, PathBuf},
};
fn files(root: &Path, out: &mut Vec<PathBuf>) {
    for e in fs::read_dir(root).unwrap() {
        let e = e.unwrap();
        if e.file_type().unwrap().is_dir() {
            files(&e.path(), out)
        } else if e.file_type().unwrap().is_file() {
            out.push(e.path());
        }
    }
}
fn add(bundle: &mut Vec<u8>, name: &str, bytes: &[u8]) {
    bundle.extend_from_slice(&(name.len() as u32).to_le_bytes());
    bundle.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    bundle.extend_from_slice(name.as_bytes());
    bundle.extend_from_slice(bytes);
}
fn main() {
    let crate_root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let checkout = crate_root.join("../..");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let source = if checkout.join("crates/rustx-core/src/lib.rs").exists() {
        checkout
    } else {
        let archive = crate_root.join("assets/framework.bin");
        println!("cargo:rerun-if-changed={}", archive.display());
        let bytes =
            fs::read(archive).expect("Run scripts/prepare-cli.mjs before packaging the CLI");
        let sdk = out.join("sdk");
        let mut offset = 0;
        while offset < bytes.len() {
            assert!(offset + 12 <= bytes.len(), "Invalid SDK header");
            let names = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
            let size =
                u64::from_le_bytes(bytes[offset + 4..offset + 12].try_into().unwrap()) as usize;
            offset += 12;
            assert!(
                names <= 4096 && size <= 32 * 1024 * 1024 && offset + names + size <= bytes.len(),
                "Invalid SDK bounds"
            );
            let name = std::str::from_utf8(&bytes[offset..offset + names]).unwrap();
            assert!(
                Path::new(name)
                    .components()
                    .all(|p| matches!(p, std::path::Component::Normal(_))),
                "Invalid SDK path"
            );
            offset += names;
            let file = sdk.join(name);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, &bytes[offset..offset + size]).unwrap();
            offset += size;
        }
        sdk
    };
    for name in [
        "host.js",
        "index.html",
        "services.js",
        "semantics.js",
        "devtools.js",
        "inspector.html",
        "inspector.js",
        "inspector.css",
        "playground.js",
        "service-worker.js",
    ] {
        let p = source.join("web").join(name);
        println!("cargo:rerun-if-changed={}", p.display());
        fs::copy(p, out.join(name)).unwrap();
    }
    for (from, to) in [
        ("fixtures/users.json", "users.json"),
        ("playground/preview.html", "preview.html"),
        ("playground/preview.js", "preview.js"),
        ("playground/worker.js", "worker.js"),
        ("docs/TUTORIAL.md", "tutorial.md"),
        ("rust-toolchain.toml", "rust-toolchain.toml"),
        ("crates/rustx-core/src/widgets.rs", "core-widgets.rs"),
        ("crates/rustx-macros/src/lib.rs", "macros.rs"),
    ] {
        let p = source.join(from);
        println!("cargo:rerun-if-changed={}", p.display());
        fs::copy(p, out.join(to)).unwrap();
    }
    let mut bundle = vec![];
    let mut paths = vec![];
    for entry in fs::read_dir(source.join("crates")).unwrap() {
        let e = entry.unwrap();
        if e.file_type().unwrap().is_dir() && e.file_name() != "rustx-cli" {
            files(&e.path(), &mut paths);
        }
    }
    for folder in ["web", "fixtures", "playground", "docs"] {
        if source.join(folder).exists() {
            files(&source.join(folder), &mut paths);
        }
    }
    // Only public authoring documentation belongs in the embedded SDK, not historical evidence.
    paths.retain(|p| {
        !p.components().any(|part| part.as_os_str() == "evidence")
            && p.extension().is_none_or(|e| e != "pdf" && e != "png")
    });
    paths.sort();
    for p in paths {
        println!("cargo:rerun-if-changed={}", p.display());
        add(
            &mut bundle,
            &p.strip_prefix(&source)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/"),
            &fs::read(p).unwrap(),
        );
    }
    for name in ["LICENSE", "NOTICE.md", "rust-toolchain.toml", "Cargo.lock"] {
        let p = source.join(name);
        println!("cargo:rerun-if-changed={}", p.display());
        add(&mut bundle, name, &fs::read(p).unwrap());
    }
    add(
        &mut bundle,
        "Cargo.toml",
        include_bytes!("templates/sdk-workspace.toml"),
    );
    fs::write(out.join("framework.bin"), bundle).unwrap();
}
