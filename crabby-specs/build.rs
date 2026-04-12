use tonic_prost_build::configure;

fn main() {
    println!("cargo:rerun-if-changed=../proto/groups.proto");
    println!("cargo:rerun-if-changed=../proto/auth.proto");

    configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(
            &["../proto/groups.proto", "../proto/auth.proto"],
            &["../proto"],
        )
        .expect("failed to compile proto files");
}
