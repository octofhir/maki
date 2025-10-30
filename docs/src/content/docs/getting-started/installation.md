---
title: Installation
description: How to install FSH Lint
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
git clone https://github.com/octofhir/maki-rs.git
cd maki-rs
cargo install --path crates/maki-cli
```

## Download Pre-built Binaries

Download the latest release for your platform from:
https://github.com/octofhir/maki-rs/releases

### macOS

```bash
curl -L https://github.com/octofhir/maki-rs/releases/latest/download/maki-macos.tar.gz | tar xz
sudo mv maki /usr/local/bin/
```

### Linux

```bash
curl -L https://github.com/octofhir/maki-rs/releases/latest/download/maki-linux.tar.gz | tar xz
sudo mv maki /usr/local/bin/
```

### Windows

Download `maki-windows.zip` from releases and add to PATH.

## Verify Installation

```bash
maki --version
# Should output: maki 0.1.0
```

## Next Steps

- [Quick Start Guide](/getting-started/quick-start/)
- [Configuration](/configuration/config-file/)
