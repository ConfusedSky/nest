extern crate bindgen;

use std::borrow::Borrow;
use std::env;
use std::fs::remove_file;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let wren_dir = format!("{manifest_dir}/lib/wren");
    let mut headers = Vec::new();
    let mut c_files = Vec::new();
    let mut inc_files = Vec::new();

    let wren_src = PathBuf::from(&wren_dir).join("src");
    let sub_dirs = ["include", "optional", "vm"];

    for dir in sub_dirs {
        let sub_dir = wren_src.join(dir);

        for file in sub_dir
            .read_dir()
            .expect("Unable to read source files")
            .flatten()
        {
            let file_path = file.path();
            let file_type = file_path.extension();
            if let Some(file_type) = file_type {
                match file_type.to_string_lossy().borrow() {
                    "c" => c_files.push(file_path),
                    "h" => headers.push(file_path),
                    "inc" => inc_files.push(file_path),
                    _ => {}
                }
            }
        }
    }

    println!("cargo:rerun-if-env-changed=WREN_DEBUG");
    println!("cargo:rerun-if-changed=headers.h");
    for file in headers.iter().chain(c_files.iter()).chain(inc_files.iter()) {
        let file = file.display();
        println!("cargo:rerun-if-changed={file}");
    }

    // let debug = true;
    let debug = {
        if let Ok(v) = env::var("WREN_DEBUG") {
            v == "true"
        } else {
            false
        }
    };

    let mut build = cc::Build::new();
    build
        .warnings(false)
        .include(format!("{wren_dir}/src/include"))
        .include(format!("{wren_dir}/src/optional"))
        .include(format!("{wren_dir}/src/vm"))
        .files(c_files.iter());
    if debug {
        build.define("DEBUG", None);
    }
    build.compile("wren");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let dir = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(dir.as_str()).join("bindings.rs");

    if out_path.exists() {
        remove_file(&out_path).expect("Unable to remove existing bindings!")
    }

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        .clang_arg(format!("-I{wren_dir}/src/include"))
        // The input header we would like to generate
        // bindings for.
        .header("headers.h")
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(out_path)
        .expect("Couldn't write bindings!");
}
