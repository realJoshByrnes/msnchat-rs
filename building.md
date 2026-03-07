# Building on Debian (cross-compiling to Windows GNU)

This project can be built on Debian for 32-bit Windows (`i686-pc-windows-gnu`) using the MinGW toolchain.

## Scope

These instructions are for Debian-based systems and produce a Windows GNU binary from Linux.

## 1) Install system dependencies (one-time)

```bash
sudo apt update
sudo apt install -y build-essential curl mingw-w64 binutils-mingw-w64-i686
```

Why these packages:
- `build-essential`: common native build tools (`gcc`, `make`, etc.)
- `curl`: downloads Rust installer
- `mingw-w64`: cross compiler for Windows GNU targets
- `binutils-mingw-w64-i686`: includes 32-bit Windows GNU binutils tools

## 2) Install Rust with `rustup` (one-time)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
```

Load Cargo environment for the current shell:

```bash
. "$HOME/.cargo/env"
```

Tip: add this line to your shell profile (`~/.bashrc` or `~/.zshrc`) so new shells pick up Cargo automatically:

```bash
. "$HOME/.cargo/env"
```

## 3) Add the Windows GNU target (one-time)

```bash
rustup target add i686-pc-windows-gnu
```

## 4) Build the project

```bash
cargo build --target i686-pc-windows-gnu
```

For optimized output:

```bash
cargo build --release --target i686-pc-windows-gnu
```

## Notes for this repository

- This repository's `.cargo/config.toml` defaults to `i686-pc-windows-msvc`.
- On Debian, always pass `--target i686-pc-windows-gnu` explicitly unless you change your local Cargo config.

## Verify toolchain state

```bash
rustup show
rustup target list --installed
```

Expected installed target includes:

```text
i686-pc-windows-gnu
```

## Common troubleshooting

- Error about missing MinGW tools:
  - Re-run: `sudo apt install -y mingw-w64 binutils-mingw-w64-i686`
- `cargo` or `rustup` not found in a new shell:
  - Run: `. "$HOME/.cargo/env"`
  - Ensure the same line is present in your shell profile.
- Built wrong target by accident:
  - Rebuild with explicit target: `cargo build --target i686-pc-windows-gnu`

## Output location

- Debug build: `target/i686-pc-windows-gnu/debug/`
- Release build: `target/i686-pc-windows-gnu/release/`
