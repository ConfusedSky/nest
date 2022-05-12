extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    const WREN_H: &str = "lib/wren/src/include/wren.h";
    println!("cargo:rerun-if-changed={}", WREN_H);

    // TODO: Actaully build this in this build script
    println!("cargo:rustc-link-search=lib/wren/lib");
    println!("cargo:rustc-link-lib=static=wren");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header(WREN_H)
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
