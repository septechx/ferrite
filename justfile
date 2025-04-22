# Set the default shell
set shell := ["sh", "-cu"]

build:
    cargo build --release

install:
    cargo install --path .

clean:
    cargo clean

run:
    cargo run

fmt:
    cargo fmt

lint:
    cargo clippy --all-targets --all-features -- -D warnings
