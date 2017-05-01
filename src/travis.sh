cargo test --verbose
cd eval
cargo run --release --example adder
cargo run --release --example filter
cargo run --release --example to_string
cargo run --release --example unique_elms
