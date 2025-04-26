# Set the default shell
set shell := ["sh", "-cu"]

install:
    cargo install --path .

build-full:
    cargo build --release
    cargo build --target x86_64-pc-windows-gnu --release

build:
    cargo build

clean:
    cargo clean

run:
    cargo run

fmt:
    cargo fmt

lint:
    cargo clippy --all-targets --all-features -- -D warnings
