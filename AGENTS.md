# Cloudroom core

- Keep one Rust package and one service; app integrations call the API.
- Run `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, and `cargo test --locked --all-targets`.
- Keep credentials and internal notes in ignored files. Review changes before publication.
