# emmylua_check_one

Check a single Lua file with full project type context using the [EmmyLua analyzer](https://github.com/EmmyLuaLs/emmylua-analyzer-rust).

## Why

`emmylua_check` checks an entire workspace. When you want to check only one file, running it directly on that file loses cross-file type information (the file becomes the only "main workspace"). `emmylua_check_one` solves this by loading the full project workspace (same as `emmylua_check`) but only diagnosing the target file.

## How it works

1. Load `.emmyrc.json` and process workspace/library paths.
2. Set up the main workspace at the project root and add all configured libraries.
3. Collect and index all Lua files in the workspace (same as `emmylua_check`).
4. Run diagnostics on **only the target file**.
5. Print results.

No temporary files, no directory copying, no source-dir guessing.

## Usage

```bash
# Auto-detect .emmyrc.json or .luarc.json (searches upward from the file)
emmylua_check_one src/path/to/File.lua

# Or specify config explicitly
emmylua_check_one -c .emmyrc.json src/path/to/File.lua
```

### Options

| Flag | Description |
|------|-------------|
| `-c, --config <PATH>` | Path to `.emmyrc.json` or `.luarc.json` (optional, auto-detected if omitted) |
| `<FILE>` | Lua file to check (required) |
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

```bash
cargo build --release
```

The binary will be at `./target/release/emmylua_check_one`.

