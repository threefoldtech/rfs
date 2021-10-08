fn main() {
    capnpc::CompilerCommand::new()
        .src_prefix("schema")
        .file("schema/schema.capnp")
        .run()
        .expect("schema compiler command");
}
