# See comment in Cargo.toml on the crate-type attribute for an explanation of why we use `cargo rustc` to build for .wasm.
build:
  RUSTFLAGS='-C target-feature=+simd128' cargo rustc --crate-type cdylib --profile release-debug --target wasm32-unknown-unknown

bench:
  cargo bench

test:
  cargo test