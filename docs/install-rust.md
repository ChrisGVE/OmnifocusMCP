# Installation

This guide installs `omnifocus-mcp`, registers it with an MCP client, and covers the errors you
are most likely to meet on first use. For what the server does, see the [README](../README.md);
for every tool and parameter, see the [tool reference](tools.md).

## Contents

- [Requirements](#requirements)
- [Install with Homebrew](#install-with-homebrew)
- [Install from a release tarball](#install-from-a-release-tarball)
- [Build from source](#build-from-source)
- [Upgrade from the upstream server](#upgrade-from-the-upstream-server)
- [Configure your MCP client](#configure-your-mcp-client)
- [Troubleshooting](#troubleshooting)

## Requirements

- macOS, with OmniFocus installed. Tested with OmniFocus 4.9.2 Pro.
- OmniFocus running whenever a tool is called.
- macOS Automation permission for the app that starts the server (your terminal, Claude Desktop,
  Cursor, and so on) to control OmniFocus. macOS asks for it on the first tool call.
- For source builds only: a Rust toolchain (`rustc`, `cargo`) from [rustup](https://rustup.rs).

## Install with Homebrew

Prebuilt binaries for Apple silicon and Intel are published with each release.

```bash
brew install ChrisGVE/tap/omnifocus-mcp
omnifocus-mcp --version
```

The second command prints `omnifocus-mcp 2.0.0`. The binary is installed as
`$(brew --prefix)/bin/omnifocus-mcp`.

## Install from a release tarball

Each [release](https://github.com/ChrisGVE/OmnifocusMCP/releases) has two archives,
`omnifocus-mcp-<version>-aarch64-apple-darwin.tar.gz` (Apple silicon) and
`omnifocus-mcp-<version>-x86_64-apple-darwin.tar.gz` (Intel), each with a `.sha256` file. For
2.0.0 on Apple silicon:

```bash
shasum -a 256 -c omnifocus-mcp-2.0.0-aarch64-apple-darwin.tar.gz.sha256
tar -xzf omnifocus-mcp-2.0.0-aarch64-apple-darwin.tar.gz
./omnifocus-mcp --version
```

Move `omnifocus-mcp` to a directory of your choice and use its absolute path in the client
configuration. The binary is not notarized; if macOS refuses to run it, see
[macOS refuses to run a downloaded binary](#macos-refuses-to-run-a-downloaded-binary).

## Build from source

```bash
git clone https://github.com/ChrisGVE/OmnifocusMCP.git
cd OmnifocusMCP/rust
cargo build --release
./target/release/omnifocus-mcp --version
```

The binary is `OmnifocusMCP/rust/target/release/omnifocus-mcp`. CI builds with the current stable
Rust release; if the build fails on an older toolchain, run `rustup update`.

## Upgrade from the upstream server

The upstream project (`vitalyrodnenko/OmnifocusMCP`) also publishes a Homebrew formula named
`omnifocus-mcp` that installs a binary named `omnifocus-mcp`. Homebrew cannot link both, so with
the upstream formula installed your client may keep starting the upstream binary. Remove the
upstream one first:

```bash
brew uninstall vitalyrodnenko/omnifocus-mcp/omnifocus-mcp
brew untap vitalyrodnenko/omnifocus-mcp
brew install ChrisGVE/tap/omnifocus-mcp
```

If your client ran the upstream Python or TypeScript server, replace that entry's `command` and
`args` with the binary, as in the next section. An entry that already runs `omnifocus-mcp` needs
no change.

Version 2.0.0 rejects parameter names a tool does not declare, which upstream silently ignored.
See the breaking changes in the [changelog](../CHANGELOG.md#200---2026-10-07).

## Configure your MCP client

### Claude Code

```bash
claude mcp add --scope user omnifocus -- omnifocus-mcp
```

For a release tarball or a source build, give the absolute path instead of `omnifocus-mcp`:

```bash
claude mcp add --scope user omnifocus -- /absolute/path/to/OmnifocusMCP/rust/target/release/omnifocus-mcp
```

### Claude Desktop, Cursor and other stdio clients

These clients take a JSON entry. An app opened from the Dock or Finder does not get your shell's
`PATH`, so always use the absolute path to the binary:

- Homebrew: the output of `echo "$(brew --prefix)/bin/omnifocus-mcp"`, usually
  `/opt/homebrew/bin/omnifocus-mcp` on Apple silicon and `/usr/local/bin/omnifocus-mcp` on Intel.
- Release tarball: the directory you moved the binary to, followed by `/omnifocus-mcp`.
- Source build: `/absolute/path/to/OmnifocusMCP/rust/target/release/omnifocus-mcp`.

```json
{
  "mcpServers": {
    "omnifocus": {
      "command": "/opt/homebrew/bin/omnifocus-mcp",
      "args": []
    }
  }
}
```

Enable only one OmniFocus MCP server per client. This server and the upstream one offer the same
tool names, so with both enabled the assistant could call either.

## Troubleshooting

### "OmniFocus is not running"

The full message is:

```text
JXA execution failed: OmniFocus is not running. Please open OmniFocus and try again.
```

Open OmniFocus and repeat the request. The server sends each call to OmniFocus without first
checking whether it is running.

### "macOS blocked Automation access to OmniFocus"

The full message is:

```text
JXA execution failed: macOS blocked Automation access to OmniFocus. Grant permission in System Settings > Privacy & Security > Automation.
```

The app that started the server is not allowed to control OmniFocus. Open System Settings >
Privacy & Security > Automation, find that app (your terminal, Claude Desktop, Cursor, ...) and
turn on OmniFocus under it.

If you denied the prompt earlier and the app is not listed, reset that app's Automation decisions
so macOS asks again. For Apple's Terminal:

```bash
tccutil reset AppleEvents com.apple.Terminal
```

Replace `com.apple.Terminal` with the bundle identifier of the app you use. Without a bundle
identifier, the command resets Automation decisions for every app.

### The client cannot start the server

A GUI client reports that the command was not found, or the server never appears. The client
cannot see your shell's `PATH`. Put the absolute path to the binary in its configuration, as
described in [Configure your MCP client](#configure-your-mcp-client).

### "unknown field" after upgrading

Since 2.0.0 a key a tool does not declare fails the call with `invalid_params` and names the
field. Common causes are camelCase identifiers (`taskId` instead of `task_id`) and camelCase
`addedAfter` / `changedAfter` (the keys are `added_after`, `changed_after`). The
[tool reference](tools.md#parameter-names) lists the exact keys.

### "Project not found" or "Folder not found"

Since 2.0.0 a `project` or `folder` value that matches nothing is an error instead of an empty
result. The value must be an id or the exact name. Use `search_projects` or `list_folders` to find
it.

### "OmniFocus did not answer within 30s"

The full message is `OmniFocus did not answer within 30s, so the outcome is unknown: a change this
call makes may still be applied. Read the object back before retrying.` One call took longer than
30 seconds, the fixed limit. The server stops waiting at that point, but OmniFocus has already
received the script and may still finish it, so a write can take effect after the error. Read the
object back (for example with `get_task` or `get_project`) before retrying a write; retrying
blindly can apply the change twice, such as a second copy of a created task. If a read is what
timed out, narrow the query: filter by project, tag or date range, or lower `limit`.

### macOS refuses to run a downloaded binary

Binaries from a release tarball are not notarized, and macOS Gatekeeper blocks such files when
they carry the quarantine attribute a browser adds to downloads. Remove the attribute from the
binary:

```bash
xattr -d com.apple.quarantine /path/to/omnifocus-mcp
```

A binary you build yourself does not carry the attribute.
