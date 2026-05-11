use std::env;

fn main() {
    let lib_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    cbindgen::Builder::new()
        .with_crate(lib_dir)
        .with_language(cbindgen::Language::C)
        .with_cpp_compat(true)
        .with_include_guard("ENERGY_BENCH_H")
        .with_no_includes()
        .with_sys_include("stdint.h")
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file("energy_bench.h");
}
