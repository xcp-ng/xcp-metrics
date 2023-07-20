extern crate prost_build;

fn main() {
    prost_build::compile_protos(&["src/openmetrics_data_model.proto"], &["src/"]).unwrap();
}
