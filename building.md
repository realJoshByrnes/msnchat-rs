
# Building on Windows (MSVC)

This project builds natively on Windows using the MSVC toolchain (32-bit target).


## 1) Install Rust (one-time)

Download and run the Rust installer from https://rust-lang.org/tools/install/

**Note:** You may need to install the [Visual Studio C++ Build tools](https://rust-lang.github.io/rustup/installation/windows-msvc.html) when prompted to do so.

## 2) Add the 32-bit MSVC target (one-time)

```
rustup target add i686-pc-windows-msvc
```

## 3) Build the project

```
cargo build --target i686-pc-windows-msvc
```

For optimized output:

```
cargo build --release --target i686-pc-windows-msvc
```

## Output location

- Debug build: `target/i686-pc-windows-msvc/debug/`
- Release build: `target/i686-pc-windows-msvc/release/`

---

# Building on Debian or Ubuntu (cross-compiling to Windows GNU)

This project can be built on Debian or Ubuntu for 32-bit Windows (`i686-pc-windows-gnu`) using the MinGW toolchain.

## Scope

These instructions are for Debian or Ubuntu systems and produce a Windows GNU binary from Linux.

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
- Always pass `--target i686-pc-windows-gnu` explicitly when not building on Windows unless you change your local Cargo config.

## Output location

- Debug build: `target/i686-pc-windows-gnu/debug/`
- Release build: `target/i686-pc-windows-gnu/release/`
