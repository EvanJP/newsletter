# CI Steps

- Testing
  - Just uses `cargo test`.
- Coverage
  - `cargo-tarpaulin` computes code coverage.
- Linting
  - `cargo clippy` is the static analysis tool.
  - `#[allow(clippy::lint_name)]`  will ignore clippy noises for a block.
- Formatting
  -  `rustfmt` is the official  rust formatter.
  -  See the `rustfmt.toml` file  for config details. Nightly features are enabled here for formatting.
- Security
  - `cargo deny` runs security vulnerabilities on crates daily. 
