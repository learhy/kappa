use std::env;

fn main() {
    capnpc::CompilerCommand::new()
        .src_prefix("schema")
        .file("schema/chf.capnp")
        .run()
        .unwrap();

    let base = env::var("CARGO_MANIFEST_DIR").unwrap();
    let target = env::var("TARGET").unwrap();

    if target.contains("darwin") {
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=Security");
        println!("cargo:rustc-link-search=native={}/libs/macos", base);
        println!("cargo:rustc-link-lib=static=pcap");
    } else if target.contains("linux-musl") {
        println!("cargo:rustc-link-search=native=/usr/local/lib");
        println!("cargo:rustc-link-search=native={}/libs/musl", base);
        println!("cargo:rustc-link-lib=static=pcap");
    }
}
