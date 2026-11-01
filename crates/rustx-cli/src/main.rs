mod server;
fn main() {
    let a: Vec<_> = std::env::args().collect();
    let result = match a.get(1).map(String::as_str) {
        Some("serve") => server::serve(
            std::path::Path::new(a.get(2).map(String::as_str).unwrap_or("dist")),
            a.get(3).map(String::as_str).unwrap_or("3000"),
        ),
        _ => {
            println!("RustX stage 1\n  cargo run -p rustx-cli -- serve dist 3000\nBuild: bash scripts/build-web.sh");
            Ok(())
        }
    };
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1)
    }
}
