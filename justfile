_default:
    just --list

build:
    cargo build

debug:
    RUST_LOG=info cargo run