# install rust implementation

## prerequisites

- macos with omnifocus installed
- omnifocus running when tools are used
- terminal/editor has macos automation permission for omnifocus
- rust toolchain (`rustc`, `cargo`) for source builds

## build from source

```bash
git clone https://github.com/ChrisGVE/OmnifocusMCP.git
cd OmnifocusMCP/rust
cargo build --release
```

the binary is `target/release/omnifocus-mcp`. optionally copy it onto your `PATH`.

verify:

```bash
./target/release/omnifocus-mcp --version
```

note: the upstream homebrew tap (`vitalyrodnenko/omnifocus-mcp`) installs the upstream build, not
this fork.

## mcp client configuration

use the absolute path to the built binary, or `omnifocus-mcp` if you copied it onto your `PATH`.
the same block works for claude desktop, cursor, and any other stdio client.

```json
{
  "mcpServers": {
    "omnifocus": {
      "command": "/absolute/path/to/OmnifocusMCP/rust/target/release/omnifocus-mcp",
      "args": []
    }
  }
}
```

## troubleshooting

### omnifocus not running

open omnifocus, then retry.

### macos automation permission denied

go to system settings -> privacy & security -> automation and allow your terminal/editor to control omnifocus.

### rust version mismatch

update rust:

```bash
rustup update
```

### macos gatekeeper blocked unsigned binary

for a binary downloaded from a github release rather than built locally:

```bash
xattr -cr /path/to/omnifocus-mcp
```
