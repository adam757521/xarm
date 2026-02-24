use std::env;
use std::fs;
use std::path::Path;

use isa_gen::emitter::strategies::latency::LatencyOptimizedCodeEmitter;
use isa_gen::emitter::traits::CodeEmitter;

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("a32.rs");

    let tokens = LatencyOptimizedCodeEmitter::emit();

    let syntax_tree = syn::parse2(tokens)
        .expect("Generated code is not valid Rust");

    fs::write(
        &dest_path,
        prettyplease::unparse(&syntax_tree)
    ).unwrap();

    println!("cargo::rerun-if-changed=build.rs");
}
