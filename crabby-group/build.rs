use tonic_prost_build::configure;

fn main() {
    // Trigger recompilation when a new migration is added
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=../proto/groups.proto");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=Cargo.toml");

    configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["../proto/groups.proto"], &["../proto"])
        .expect("failed to compile groups.proto");
}
