use std::fs;
use std::path::Path;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let dist_dir = Path::new(&manifest_dir).join("../../apps/web/dist");
    if !dist_dir.exists() {
        let _ = fs::create_dir_all(&dist_dir);
    }
    println!("cargo:rerun-if-changed={}", dist_dir.display());
}
