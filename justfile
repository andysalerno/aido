_default:
    just --list

build:
    cargo build

build-release:
    cargo build --release

debug:
    RUST_LOG=info cargo run

fix:
    cargo clippy --fix