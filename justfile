install:
    cargo install --path .

build-full:
    cargo build --release
    cargo build --target x86_64-pc-windows-gnu --release
    mkdir build
    cp target/release/ferrite build/ferrite-linux-x86_64
    cp target/x86_64-pc-windows-gnu/release/ferrite.exe build/ferrite-windows-x86_64.exe

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
