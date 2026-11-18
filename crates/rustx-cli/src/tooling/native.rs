//! Host-native, relocatable application packages. No cross-build is execution evidence.
use super::{hash, output, run, write};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

fn child_output(root: &Path, out: &Path) -> Result<(), String> {
    if out == root
        || !out.starts_with(root)
        || out
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
        || out
            .ancestors()
            .any(|p| fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink()))
    {
        return Err("Package output must be a real child directory of the project".into());
    }
    if out.exists() {
        return Err("Package output already exists; choose a new --out directory (existing packages are never overwritten)".into());
    }
    Ok(())
}
struct Staging(PathBuf);
impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn application(root: &Path) -> Result<(Value, String), String> {
    let metadata: Value = serde_json::from_str(&output(
        "cargo",
        &["metadata", "--no-deps", "--format-version", "1", "--locked"],
        root,
    )?)
    .map_err(|e| e.to_string())?;
    select_application(root, &metadata)
}
fn select_application(root: &Path, metadata: &Value) -> Result<(Value, String), String> {
    let packages = metadata["packages"]
        .as_array()
        .ok_or("Cargo returned no packages")?;
    let app = packages
        .iter()
        .find(|p| {
            Path::new(p["manifest_path"].as_str().unwrap_or(""))
                .parent()
                .and_then(|p| p.canonicalize().ok())
                .is_some_and(|p| p == root)
        })
        .or_else(|| packages.iter().find(|p| p["name"] == "rustx-lab"))
        .ok_or("Run package from an application directory or the RustX repository root")?
        .clone();
    let bins: Vec<_> = app["targets"]
        .as_array()
        .ok_or("No Cargo targets")?
        .iter()
        .filter(|t| {
            t["kind"]
                .as_array()
                .is_some_and(|k| k.iter().any(|x| x == "bin"))
        })
        .filter_map(|t| t["name"].as_str())
        .collect();
    let bin = bins
        .iter()
        .find(|b| Some(**b) == app["default_run"].as_str())
        .or_else(|| bins.iter().find(|b| Some(**b) == app["name"].as_str()))
        .copied()
        .or_else(|| (bins.len() == 1).then(|| bins[0]))
        .ok_or("Choose the application binary with package.default-run in Cargo.toml")?
        .to_owned();
    Ok((app, bin))
}
fn dependencies(text: &str, mac: bool) -> Result<Vec<String>, String> {
    let deps: Vec<String> = if mac {
        text.lines()
            .filter_map(|l| {
                l.trim()
                    .split_once(" (compatibility version")
                    .map(|(p, _)| p.to_owned())
            })
            .collect()
    } else {
        text.lines()
            .map(str::trim)
            .filter(|l| {
                l.to_ascii_lowercase().ends_with(".dll") || l.to_ascii_lowercase().ends_with(".drv")
            })
            .map(str::to_owned)
            .collect()
    };
    if deps.is_empty() {
        return Err(
            "Dependency inspection returned no libraries; refusing an unaudited package".into(),
        );
    }
    for dep in &deps {
        let allowed = if mac {
            dep.starts_with("/System/Library/") || dep.starts_with("/usr/lib/")
        } else {
            // Imported OS DLLs only. Third-party/CRT DLLs require an explicit distribution design.
            matches!(
                dep.to_ascii_lowercase().as_str(),
                "kernel32.dll"
                    | "kernelbase.dll"
                    | "ntdll.dll"
                    | "user32.dll"
                    | "gdi32.dll"
                    | "advapi32.dll"
                    | "ole32.dll"
                    | "oleaut32.dll"
                    | "combase.dll"
                    | "shell32.dll"
                    | "shlwapi.dll"
                    | "bcrypt.dll"
                    | "bcryptprimitives.dll"
                    | "ws2_32.dll"
                    | "userenv.dll"
                    | "comdlg32.dll"
                    | "comctl32.dll"
                    | "dbghelp.dll"
                    | "uxtheme.dll"
                    | "dwmapi.dll"
                    | "imm32.dll"
                    | "dwrite.dll"
                    | "d3d12.dll"
                    | "dxgi.dll"
                    | "d3dcompiler_47.dll"
                    | "opengl32.dll"
                    | "crypt32.dll"
                    | "secur32.dll"
                    | "winmm.dll"
                    | "powrprof.dll"
                    | "cfgmgr32.dll"
                    | "version.dll"
                    | "setupapi.dll"
                    | "propsys.dll"
                    | "winspool.drv"
                    | "uiautomationcore.dll"
                    | "oleacc.dll"
                    | "shcore.dll"
                    | "iphlpapi.dll"
                    | "winhttp.dll"
                    | "wintrust.dll"
                    | "ncrypt.dll"
                    | "psapi.dll"
                    | "dcomp.dll"
                    | "avrt.dll"
            ) || dep.to_ascii_lowercase().starts_with("api-ms-win-")
                || dep.to_ascii_lowercase().starts_with("ext-ms-win-")
        };
        if !allowed {
            return Err(format!(
                "Non-system dependency {dep}; bundle/resolve it explicitly before packaging"
            ));
        }
    }
    Ok(deps)
}
fn payload_files(root: &Path, at: &Path, result: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(at).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() {
            return Err("Symlinks are not allowed in package payloads".into());
        }
        if kind.is_dir() {
            payload_files(root, &entry.path(), result)?;
        } else {
            result.push(entry.path().strip_prefix(root).unwrap().to_owned());
        }
    }
    Ok(())
}
fn collect_licenses(root: &Path, target: &str, destination: &Path) -> Result<(), String> {
    let metadata: Value = serde_json::from_str(&output(
        "cargo",
        &[
            "metadata",
            "--locked",
            "--format-version",
            "1",
            "--filter-platform",
            target,
        ],
        root,
    )?)
    .map_err(|e| e.to_string())?;
    let mut report = Vec::new();
    for package in metadata["packages"]
        .as_array()
        .ok_or("Cargo omitted license metadata")?
    {
        let directory = Path::new(
            package["manifest_path"]
                .as_str()
                .ok_or("Missing dependency manifest")?,
        )
        .parent()
        .ok_or("Missing dependency directory")?;
        let name = package["name"].as_str().ok_or("Missing dependency name")?;
        let version = package["version"]
            .as_str()
            .ok_or("Missing dependency version")?;
        let identity = hash(package["id"].as_str().unwrap_or(name));
        let label = format!("{name}-{version}-{}", &identity[..8]);
        let explicit = package["license_file"].as_str().map(|p| directory.join(p));
        let mut paths = Vec::new();
        for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let upper = entry.file_name().to_string_lossy().to_ascii_uppercase();
            if entry.file_type().map_err(|e| e.to_string())?.is_file()
                && ["LICENSE", "LICENCE", "COPYING", "NOTICE"]
                    .iter()
                    .any(|prefix| upper.starts_with(prefix))
            {
                paths.push(entry.path());
            }
        }
        if let Some(path) = explicit {
            if fs::symlink_metadata(&path).is_ok_and(|m| m.is_file() && !m.file_type().is_symlink())
                && path
                    .canonicalize()
                    .is_ok_and(|p| directory.canonicalize().is_ok_and(|d| p.starts_with(d)))
                && !paths.contains(&path)
            {
                paths.push(path);
            }
        }
        paths.sort();
        let mut copied = Vec::new();
        for (index, path) in paths.iter().enumerate() {
            let relative = format!(
                "dependencies/{label}/{index}-{}",
                path.file_name().unwrap().to_string_lossy()
            );
            write(
                &destination.join(&relative),
                fs::read(path).map_err(|e| e.to_string())?,
            )?;
            copied.push(relative);
        }
        report.push(json!({"name":name,"version":version,"license":package["license"],"files":copied,"review_needed":copied.is_empty() || package["license"].is_null()}));
    }
    write(&destination.join("DEPENDENCIES.json"), serde_json::to_vec_pretty(&json!({"scope":"Cargo resolved graph, including build and non-runtime packages; not a legal compliance determination", "packages":report})).unwrap())
}

fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn bundle(root: &Path, target: &str, out: &Path) -> Result<(), String> {
    child_output(root, out)?;
    if !["native", "windows", "macos"].contains(&target) {
        return Err("Package target must be native, windows or macos".into());
    }
    if target == "windows" && !cfg!(windows) || target == "macos" && !cfg!(target_os = "macos") {
        return Err(
            "Package on the actual requested OS; cross-compilation is not native acceptance".into(),
        );
    }
    if !cfg!(any(windows, target_os = "macos")) {
        return Err("Native packaging currently supports macOS and Windows; Linux can still use build --target native".into());
    }
    let rust = output("rustc", &["-vV"], root)?;
    let triple = rust
        .lines()
        .find_map(|l| l.strip_prefix("host: "))
        .ok_or("rustc omitted host triple")?;
    if cfg!(windows) && !triple.ends_with("-pc-windows-msvc") {
        return Err("Windows packages require the MSVC host toolchain".into());
    }
    // Require native inspection/archive tools before spending time compiling.
    if cfg!(windows) {
        // MSVC's help command can exit unsuccessfully even when dumpbin works.
        // Inspect this CLI's actual PE headers instead of treating help as a health check.
        let cli = std::env::current_exe().map_err(|e| e.to_string())?;
        output(
            "dumpbin",
            &["/HEADERS", cli.to_str().ok_or("Non-UTF8 CLI path")?],
            root,
        )
        .map_err(|e| format!("Use an MSVC developer shell (dumpbin required): {e}"))?;
    }
    let (app, bin) = application(root)?;
    let name = app["name"].as_str().ok_or("Missing package name")?;
    let version = app["version"].as_str().ok_or("Missing version")?;
    let mut command = Command::new("cargo");
    command
        .args([
            "build",
            "--locked",
            "--release",
            "-p",
            name,
            "--bin",
            &bin,
            "--target",
            triple,
            "--message-format=json-render-diagnostics",
        ])
        .current_dir(root)
        .stderr(Stdio::inherit());
    if cfg!(windows) {
        let mut flags = std::env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or_else(|_| {
            std::env::var("RUSTFLAGS")
                .unwrap_or_default()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join("\x1f")
        });
        if !flags.is_empty() {
            flags.push('\x1f');
        }
        flags.push_str("-C\x1ftarget-feature=+crt-static");
        command.env("CARGO_ENCODED_RUSTFLAGS", flags);
    }
    let built = command.output().map_err(|e| e.to_string())?;
    if !built.status.success() {
        return Err(
            "Native application build failed; existing package output was not changed".into(),
        );
    }
    let binary = String::from_utf8_lossy(&built.stdout)
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .find_map(|v| {
            if v["reason"] == "compiler-artifact" && v["target"]["name"] == bin {
                v["executable"].as_str().map(PathBuf::from)
            } else {
                None
            }
        })
        .ok_or("Cargo did not report the selected executable")?;
    let mac = cfg!(target_os = "macos");
    let audit = if mac {
        output(
            "otool",
            &["-L", binary.to_str().ok_or("Non-UTF8 executable path")?],
            root,
        )?
    } else {
        output(
            "dumpbin",
            &[
                "/DEPENDENTS",
                binary.to_str().ok_or("Non-UTF8 executable path")?,
            ],
            root,
        )?
    };
    let libraries =
        dependencies(&audit, mac).map_err(|e| format!("{e}\nDependency inspection:\n{audit}"))?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let staging_path = out.with_extension(format!("rustx-{}-{stamp}", std::process::id()));
    fs::create_dir_all(staging_path.parent().ok_or("Missing output parent")?)
        .map_err(|e| e.to_string())?;
    fs::create_dir(&staging_path).map_err(|e| e.to_string())?;
    let stage = Staging(staging_path);
    let stem = format!("{name}-{version}-{triple}");
    let payload = stage.0.join(&stem);
    let executable = if mac {
        format!("{name}.app/Contents/MacOS/{bin}")
    } else {
        format!("{bin}.exe")
    };
    write(&payload.join("DEPENDENCIES.txt"), &audit)?;
    let dest = payload.join(&executable);
    fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::copy(&binary, &dest).map_err(|e| e.to_string())?;
    if mac {
        let numeric = version.split(['-', '+']).next().unwrap();
        let plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>{}</string>
<key>CFBundleIdentifier</key><string>org.rustx.{}</string>
<key>CFBundleName</key><string>{}</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>{}</string>
<key>CFBundleVersion</key><string>{}</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
"#,
            xml(&bin),
            xml(&name.replace('_', "-").to_ascii_lowercase()),
            xml(name),
            xml(numeric),
            xml(numeric)
        );
        write(
            &payload.join(format!("{name}.app/Contents/Info.plist")),
            plist,
        )?;
    }
    let framework = if root.join("vendor/crates/rustx-engine/assets").is_dir() {
        root.join("vendor")
    } else if root.join("crates/rustx-engine/assets").is_dir() {
        root.to_owned()
    } else {
        super::framework(root, None)?
    };
    let licenses = if mac {
        payload.join(format!("{name}.app/Contents/Resources/licenses"))
    } else {
        payload.join("licenses")
    };
    for file in ["LICENSE", "NOTICE.md"] {
        write(
            &licenses.join(format!("RustX-{file}")),
            fs::read(framework.join(file)).map_err(|e| e.to_string())?,
        )?;
    }
    for file in ["FONT-LICENSE.txt", "NOTO-EMOJI-LICENSE.txt"] {
        write(
            &licenses.join(file),
            fs::read(framework.join("crates/rustx-engine/assets").join(file))
                .map_err(|e| e.to_string())?,
        )?;
    }
    for file in ["LICENSE", "LICENSE.md", "LICENSE.txt", "NOTICE.md"] {
        if root.join(file).is_file() {
            write(
                &licenses.join(format!("Application-{file}")),
                fs::read(root.join(file)).map_err(|e| e.to_string())?,
            )?;
        }
    }
    write(&payload.join("RUN.md"), format!("# {name} {version}\n\n{}\n\nNo Rust, Cargo, Node or browser runtime is needed to launch this executable. Fonts are embedded. The recipient still needs the matching OS/architecture and its system libraries. This is an unsigned portable developer package, not an installer or notarized/SmartScreen-approved release. Do not bypass OS protections; public distribution requires signing/release review.\n\nOnly embedded assets are supported by this first packaging contract. Application-specific external files/services are not automatically bundled. Application authors must review dependency licenses before redistribution.\n\n`DEPENDENCIES.txt` records direct link dependencies, not dynamically loaded GPU drivers or a complete clean-machine audit. Platform interaction, IME/accessibility and GPU verification remain separate gates.\n", if mac { format!("Open `{name}.app` locally, or run `{executable}` from a terminal.") } else { format!("Extract the entire ZIP and run `{executable}`.") }))?;
    collect_licenses(root, triple, &licenses)?;
    let mut paths = Vec::new();
    payload_files(&payload, &payload, &mut paths)?;
    paths.sort();
    let mut hashes = serde_json::Map::new();
    for path in paths {
        hashes.insert(
            path.to_string_lossy().replace('\\', "/"),
            json!(hash(
                fs::read(payload.join(path)).map_err(|e| e.to_string())?
            )),
        );
    }
    let manifest = json!({"format": "rustx-native-package-v1", "name": name, "version":version, "target":triple, "executable":executable, "rust":rust, "static_crt":cfg!(windows), "dependencies":libraries, "files":hashes, "verification":"Built and dependency-inspected; execution/clean-machine acceptance must be recorded separately", "signing":"No distribution signing or notarization performed"});
    write(
        &payload.join("package.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )?;
    let manifest_digest = hash(fs::read(payload.join("package.json")).map_err(|e| e.to_string())?);
    write(
        &payload.join("SHA256SUMS.txt"),
        format!(
            "{}{}  package.json\n",
            hashes
                .iter()
                .map(|(p, h)| format!("{}  {p}\n", h.as_str().unwrap()))
                .collect::<String>(),
            manifest_digest
        ),
    )?;
    let archive = format!("{stem}.zip");
    if mac {
        run(
            "ditto",
            &["-c", "-k", "--keepParent", &stem, &archive],
            &stage.0,
        )?;
    } else {
        // Use the inbox libarchive tool. Relative entry names keep the staging
        // path (which can have a Windows verbatim prefix) out of the ZIP.
        let tar = PathBuf::from(std::env::var_os("SystemRoot").ok_or("Missing SystemRoot")?)
            .join("System32")
            .join("tar.exe");
        run(
            tar.to_str().ok_or("Non-UTF8 Windows system path")?,
            &["--format", "zip", "-cf", &archive, "--", &stem],
            &stage.0,
        )?;
    }
    write(
        &stage.0.join("SHA256SUMS.txt"),
        format!(
            "{}  {archive}\n",
            hash(fs::read(stage.0.join(&archive)).map_err(|e| e.to_string())?)
        ),
    )?;
    // Output must still be absent after the build. Never merge with an existing directory.
    child_output(root, out)?;
    fs::rename(&stage.0, out).map_err(|e| e.to_string())?;
    println!(
        "Packaged {}\nLaunch: {}\nArchive: {}",
        out.display(),
        out.join(&stem).join(executable).display(),
        out.join(archive).display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn application_matches_canonical_project_directory() {
        let fixture = std::env::temp_dir().join(format!(
            "rustx-native-selection-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&fixture).unwrap();
        let stage = Staging(fixture);
        let root = stage.0.canonicalize().unwrap();
        // Cargo can report C:\\... while current_dir canonicalizes to \\\\?\\C:\\...
        // on Windows. Exercise another spelling of the same directory on every OS.
        let alternate = stage.0.join("..").join(stage.0.file_name().unwrap());
        let metadata = json!({"packages":[{
            "name":"independent-app",
            "manifest_path":alternate.join("Cargo.toml"),
            "targets":[{"name":"independent-app","kind":["bin"]}]
        }]});
        let (app, bin) = select_application(&root, &metadata).unwrap();
        assert_eq!(app["name"], "independent-app");
        assert_eq!(bin, "independent-app");
    }
    #[test]
    fn dependency_audit_rejects_unbundled_libraries() {
        assert!(dependencies("app:\n /usr/lib/libSystem.B.dylib (compatibility version 1.0.0, current version 1.0.0)", true).is_ok());
        assert!(dependencies("app:\n /opt/homebrew/lib/libcustom.dylib (compatibility version 1.0.0, current version 1.0.0)", true).is_err());
        assert!(dependencies(" KERNEL32.dll\n USER32.dll", false).is_ok());
        assert!(dependencies(" KERNEL32.dll\n VCRUNTIME140.dll", false).is_err());
        assert!(dependencies(" KERNEL32.dll\n custom.dll", false).is_err());
        assert!(dependencies("missing output", false).is_err());
    }
    #[test]
    fn dependency_audit_accepts_recorded_windows_system_imports() {
        // Actual Windows CI import report from d80ef49 (run 43387127048).
        let report = include_str!("fixtures/windows-dependents.txt");
        assert_eq!(dependencies(report, false).unwrap().len(), 19);
        assert!(dependencies(&format!("{report}\n VCRUNTIME140.dll"), false).is_err());
    }
    #[test]
    fn package_output_rejects_escape() {
        assert!(child_output(Path::new("/project"), Path::new("/project")).is_err());
        assert!(child_output(Path::new("/project"), Path::new("/outside")).is_err());
        assert!(child_output(Path::new("/project"), Path::new("/project/../outside")).is_err());
    }
}
