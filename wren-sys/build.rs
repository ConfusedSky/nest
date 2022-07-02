extern crate bindgen;

use std::env;
use std::path::Path;
use std::process::Command;
use which::which;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let wren_dir = format!("{}/lib/wren", manifest_dir);
    let wren_h = format!("{}/src/include/wren.h", wren_dir);
    let wren_c = format!("{}/lib/wren.c", wren_dir);

    println!("cargo:rerun-if-changed={}", wren_h);
    println!("cargo:rerun-if-changed={}", wren_c);
    println!("cargo:rerun-if-env-changed=WREN_DEBUG");

    let wren_c_path = Path::new(wren_c.as_str());

    if !wren_c_path.exists() {
        let script = format!("{}/util/generate_amalgamation.py", wren_dir);
        let python = which("python3").expect("wren_sys requires python to generate almalgamation!");
        let result = Command::new(python)
            .current_dir(wren_dir)
            .arg(script)
            .output()
            .expect("Amalgamation script failed to run");
        std::fs::write(wren_c_path, &result.stdout).expect("Failed writing wren.c");
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
    build.file(wren_c_path).warnings(false);
    if debug {
        build.define("DEBUG", None);
    }
    build.compile("wren");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header(wren_h)
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let dir = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(dir.as_str());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
