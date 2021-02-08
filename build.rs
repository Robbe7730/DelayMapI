fn main() {
    protoc_rust::Codegen::new()
        .out_dir("src")
        .inputs(&["protos/gtfs-realtime.proto"])
        .include("protos")
        .run()
        .expect("protoc");
}
