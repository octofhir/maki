---
title: Troubleshooting
description: Common issues and solutions
---

Solutions to common problems when using FSH Lint.

## Installation Issues

### Cargo Install Fails

**Problem**: `cargo install fsh-lint` fails

**Solutions**:

1. Update Rust:
```bash
rustup update stable
```

2. Clear cargo cache:
```bash
rm -rf ~/.cargo/registry
cargo install fsh-lint
```

3. Build with verbose output:
```bash
cargo install fsh-lint -v
```

### Binary Not Found

**Problem**: `fsh-lint: command not found`

**Solution**: Add Cargo bin to PATH:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Add to `.bashrc` or `.zshrc` for persistence.

## Configuration Issues

### Config File Not Found

**Problem**: FSH Lint can't find configuration

**Solution**: Create config in project root:

```bash
fsh-lint init
```

### Invalid Configuration

**Problem**: Configuration file is invalid

**Solution**: Validate against schema:

```bash
fsh-lint check --config fsh-lint.json
```

## Linting Issues

### Too Many Diagnostics

**Problem**: Overwhelming number of errors

**Solutions**:

1. Start with errors only:
```bash
fsh-lint lint --severity error **/*.fsh
```

2. Fix automatically:
```bash
fsh-lint lint --fix **/*.fsh
```

3. Temporarily disable rules:
```jsonc
{
  "linter": {
    "rules": {
      "style": "off"
    }
  }
}
```

### False Positives

**Problem**: Rule incorrectly reports issue

**Solutions**:

1. Disable inline:
```fsh
// fsh-lint-disable-next-line rule-name
Profile: MyProfile
```

2. Adjust rule severity:
```jsonc
{
  "linter": {
    "rules": {
      "style/naming-convention": "off"
    }
  }
}
```

## Performance Issues

### Slow Linting

**Problem**: Linting takes too long

**Solutions**:

1. Exclude large directories:
```jsonc
{
  "files": {
    "exclude": ["**/node_modules/**", "**/build/**"]
  }
}
```

2. Limit file scope:
```bash
fsh-lint lint input/fsh/ --ignore-pattern "**/*.generated.fsh"
```

### High Memory Usage

**Problem**: FSH Lint uses too much memory

**Solution**: Process files in batches:

```bash
find . -name "*.fsh" -print0 | xargs -0 -n 10 fsh-lint lint
```

## CI/CD Issues

### Build Timeout

**Problem**: CI builds timeout during installation

**Solution**: Use cached binaries:

```yaml
- name: Download FSH Lint
  run: |
    curl -L https://github.com/octofhir/fsh-lint-rs/releases/latest/download/fsh-lint-linux.tar.gz | tar xz
    sudo mv fsh-lint /usr/local/bin/
```

### Inconsistent Results

**Problem**: Different results in CI vs local

**Solution**: Pin FSH Lint version:

```bash
cargo install fsh-lint --version 0.1.0
```

## Output Issues

### No Color in CI

**Problem**: CI output lacks color

**Solution**: Force color output:

```bash
fsh-lint --color always lint **/*.fsh
```

### Garbled Output

**Problem**: Output has encoding issues

**Solution**: Set UTF-8 encoding:

```bash
export LANG=en_US.UTF-8
fsh-lint lint **/*.fsh
```

## Getting Help

If you're still stuck:

1. Check [GitHub Issues](https://github.com/octofhir/fsh-lint-rs/issues)
2. Search [Discussions](https://github.com/octofhir/fsh-lint-rs/discussions)
3. Open a new issue with:
   - FSH Lint version (`fsh-lint --version`)
   - OS and version
   - Minimal reproduction example
   - Full error output

## Common Error Messages

### "Failed to parse FSH file"

**Cause**: Invalid FSH syntax

**Solution**: Check FSH syntax against [FSH spec](https://hl7.org/fhir/uv/shorthand/)

### "Circular dependency detected"

**Cause**: Profile inherits from itself

**Solution**: Review parent relationships

### "Unknown rule"

**Cause**: Referenced rule doesn't exist

**Solution**: Run `fsh-lint rules` to see available rules
