---
title: Installation
description: How to install MAKI
---

## Prerequisites

- Rust 1.80 or later (for building from source)
- Or use pre-built binaries

## Install from Crates.io (Recommended)

```bash
cargo install maki
```

Verify installation:

```bash
maki --version
```

## Install from Source

```bash
git clone https://github.com/octofhir/maki.git
cd maki
cargo build --release --bin maki
# Binary will be at: target/release/maki
```

Or install directly:

```bash
cargo install --path crates/maki-cli
```

## Download Pre-built Binaries

Download the latest release for your platform from:
[GitHub Releases](https://github.com/octofhir/maki/releases)

### macOS

```bash
# Apple Silicon (M1/M2/M3)
curl -L https://github.com/octofhir/maki/releases/latest/download/maki-macos-arm64 -o maki
chmod +x maki
sudo mv maki /usr/local/bin/

# Intel
curl -L https://github.com/octofhir/maki/releases/latest/download/maki-macos-x64 -o maki
chmod +x maki
sudo mv maki /usr/local/bin/
```

### Linux

```bash
curl -L https://github.com/octofhir/maki/releases/latest/download/maki-linux-x64 -o maki
chmod +x maki
sudo mv maki /usr/local/bin/
```

### Windows

Download `maki-windows-x64.exe` or `maki-windows-arm64.exe` from the releases page and add to PATH.

## Verify Installation

```bash
maki --version
```

Test with a simple command:

```bash
maki --help
```

## Next Steps

- [Quick Start Guide](/maki/getting-started/quick-start/)
- [CLI Commands Reference](/maki/cli/commands/)
- [Configuration](/maki/configuration/config-file/)
