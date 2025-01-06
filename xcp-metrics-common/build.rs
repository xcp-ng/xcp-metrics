fn main() {
    #[cfg(feature = "openmetrics")]
    prost_build::compile_protos(&["src/openmetrics_data_model.proto"], &["src/"]).unwrap();
}
