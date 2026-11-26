use std::path::{Path, PathBuf};

fn main() {
    let root = PathBuf::from("vendor/cubiomes");
    let mut sources = Vec::new();
    collect_c_sources(&root, &mut sources);
    sources.push(PathBuf::from("src/ffi/shim.c"));

    let mut build = cc::Build::new();
    for source in &sources {
        build.file(source);
    }
    build
        .include(&root)
        .include(root.join("features"))
        .include(root.join("loot"))
        .include(root.join("loot/cjson"))
        .include("src/ffi")
        .std("gnu11")
        .flag_if_supported("-mpopcnt")
        .warnings(false)
        .compile("cubiomes");

    println!("cargo:rerun-if-changed=src/ffi/shim.c");
    println!("cargo:rerun-if-changed=vendor/cubiomes");

    tauri_build::build()
}

/// Collects every C file of the vendored library, skipping its own tests.
fn collect_c_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => panic!("vendor/cubiomes ist nicht lesbar ({}): {err}", dir.display()),
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => panic!("vendor/cubiomes ist nicht lesbar: {err}"),
        };
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "tests") {
                continue;
            }
            collect_c_sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "c") {
            // `tests.c` is a standalone test runner, not part of the library.
            // It includes POSIX headers that do not exist on Windows.
            if path.file_name().is_some_and(|name| name == "tests.c") {
                continue;
            }
            out.push(path);
        }
    }
}
