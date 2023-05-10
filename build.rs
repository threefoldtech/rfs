fn main() {
    println!(
        "cargo:rustc-env=GIT_VERSION={}",
        git_version::git_version!(args = ["--tags", "--always", "--dirty=-modified"])
    );

    capnpc::CompilerCommand::new()
        .src_prefix("schema")
        .file("schema/schema.capnp")
        .run()
        .expect("schema compiler command");
}
