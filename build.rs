use std::fs;
use std::path::Path;

fn main() {
    let schema_dir = Path::new("schemas");

    let mut entries: Vec<_> = fs::read_dir(schema_dir)
        .expect("failed to read schemas/ dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "cm"))
        .map(|e| e.path())
        .collect();

    entries.sort();

    let mut hash: u64 = 0;
    for path in &entries {
        let contents = fs::read(path).unwrap_or_else(|_| panic!("failed to read {path:?}"));
        for &b in &contents {
            hash = hash.wrapping_mul(31).wrapping_add(b as u64);
        }
        for b in path.to_string_lossy().bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(b as u64);
        }

        println!("cargo:rerun-if-changed={}", path.display());
    }

    println!("cargo:rerun-if-changed={}", schema_dir.display());

    println!("cargo:rustc-env=SCHEMAS_HASH={:x}", hash);
}
