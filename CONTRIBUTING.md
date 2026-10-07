# Contributing to OmniFocus MCP

Thanks for contributing. Keep changes small, focused, and test-backed.

## Development setup

```bash
git clone https://github.com/ChrisGVE/OmnifocusMCP.git
cd OmnifocusMCP/rust
cargo check
```

Detailed install guide: `docs/install-rust.md`

## Running tests

```bash
cd rust && cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

`cargo test` runs the mocked unit tests and never touches OmniFocus. Integration tests
(`cargo test --features integration`) and `rust/examples/` act on a live OmniFocus database:
they need OmniFocus running and macOS Automation permission, and they create and delete real
items, so run them only against a database you can afford to modify.

## Key rules

- all user input must be escaped through `escape_for_jxa`
- every bug fix starts with a failing unit test that reproduces it

## Pull request guidelines

- one concern per PR, small diff preferred
- include a short rationale and testing notes
- fmt, clippy, and tests must pass before review
