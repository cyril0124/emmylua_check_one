# emmylua_check_one

Check a single Lua file (or all Lua files under a directory) with full project type context using the [EmmyLua analyzer](https://github.com/EmmyLuaLs/emmylua-analyzer-rust).

## Why

`emmylua_check` checks an entire workspace. When you want to check only one file, running it directly on that file loses cross-file type information (the file becomes the only "main workspace"). `emmylua_check_one` solves this by loading the full project workspace (same as `emmylua_check`) but only diagnosing the target file(s).

## How it works

1. Load `.emmyrc.json` and process workspace/library paths.
2. Set up the main workspace at the project root and add all configured libraries.
3. Collect and index all Lua files in the workspace (same as `emmylua_check`).
4. Run diagnostics on **only the target file(s)**.
5. Print results.

No temporary files, no directory copying, no source-dir guessing.

## Usage

```bash
# Check a single file (auto-detect .emmyrc.json or .luarc.json)
emmylua_check_one src/path/to/File.lua

# Check all .lua files under a directory
emmylua_check_one src/path/to/module/

# Or specify config explicitly
emmylua_check_one -c .emmyrc.json src/path/to/File.lua
```

### Options

| Flag | Description |
|------|-------------|
| `-c, --config <PATH>` | Path to `.emmyrc.json` or `.luarc.json` (optional, auto-detected if omitted) |
| `<FILE>` | Lua file or directory to check (required) |
| `-i, --ignore <PATTERNS>` | Comma-separated glob ignore patterns |
| `--warnings-as-errors` | Treat warnings as errors |
| `--verbose` | Verbose output |

## Install

### Pre-built binary (recommended)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cyril0124/emmylua_check_one/master/install.sh | bash
```

Or with a specific version:

```bash
VERSION=0.1.0 curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cyril0124/emmylua_check_one/master/install.sh | bash
```

Or install to a custom directory:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cyril0124/emmylua_check_one/master/install.sh | INSTALL_DIR=/usr/local/bin bash
```

### From source

#### Using `cargo install` (requires Rust toolchain)

Install directly from the GitHub repository (this project is not published to crates.io):

```bash
cargo install --git https://github.com/cyril0124/emmylua_check_one.git
```

Or install from a local clone:

```bash
git clone https://github.com/cyril0124/emmylua_check_one.git
cd emmylua_check_one
cargo install --path .
```

#### Build locally

```bash
cargo build --release
```

The binary will be at `./target/release/emmylua_check_one`.

