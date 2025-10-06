---
title: Installation
description: How to install FSH Lint
---

## Prerequisites

- Rust 1.80 or later (for building from source)
- Or use pre-built binaries

## Install from Crates.io (Recommended)

```bash
cargo install fsh-lint
```

Verify installation:

```bash
fsh-lint --version
```

## Install from Source

```bash
git clone https://github.com/octofhir/fsh-lint-rs.git
cd fsh-lint-rs
cargo install --path crates/fsh-lint-cli
```

## Download Pre-built Binaries

Download the latest release for your platform from:
https://github.com/octofhir/fsh-lint-rs/releases

### macOS

```bash
curl -L https://github.com/octofhir/fsh-lint-rs/releases/latest/download/fsh-lint-macos.tar.gz | tar xz
sudo mv fsh-lint /usr/local/bin/
```

### Linux

```bash
curl -L https://github.com/octofhir/fsh-lint-rs/releases/latest/download/fsh-lint-linux.tar.gz | tar xz
sudo mv fsh-lint /usr/local/bin/
```

### Windows

Download `fsh-lint-windows.zip` from releases and add to PATH.

## Verify Installation

```bash
fsh-lint --version
# Should output: fsh-lint 0.1.0
```

## Next Steps

- [Quick Start Guide](/getting-started/quick-start/)
- [Configuration](/configuration/config-file/)
