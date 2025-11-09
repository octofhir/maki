---
title: CI/CD Integration
description: Integrate FSH Lint with your CI/CD pipeline
---

Integrate FSH Lint into your continuous integration and deployment workflows.

## GitHub Actions

### Basic Workflow

```yaml
name: Lint FSH Files

on: [push, pull_request]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install FSH Lint
        run: cargo install maki

      - name: Check formatting
        run: maki format --check **/*.fsh

      - name: Lint FSH files
        run: maki lint **/*.fsh
```

### Format Check Only

Check formatting without linting (useful for separate jobs):

```yaml
name: Format Check

on: [push, pull_request]

jobs:
  format:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install FSH Lint
        run: cargo install maki

      - name: Check formatting
        run: maki format --check **/*.fsh
```

### Auto-format and Auto-fix

Automatically format and fix issues, then commit:

```yaml
name: Auto-format and Lint

on: [push, pull_request]

jobs:
  auto-fix:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/bin/maki
            ~/.cargo/registry
            ~/.cargo/git
          key: ${{ runner.os }}-cargo-maki

      - name: Install FSH Lint
        run: cargo install maki || true

      - name: Format FSH files
        run: maki format **/*.fsh

      - name: Lint with fixes
        run: maki lint --fix **/*.fsh

      - name: Commit changes
        uses: stefanzweifel/git-auto-commit-action@v5
        with:
          commit_message: "style: auto-format and fix FSH issues"
```

### Matrix Testing

```yaml
name: Lint FSH Files

on: [push, pull_request]

jobs:
  lint:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install maki
      - run: maki lint **/*.fsh
```

## GitLab CI

### Basic Pipeline

```yaml
lint:
  image: rust:latest
  before_script:
    - cargo install maki
  script:
    - maki lint **/*.fsh
  only:
    - merge_requests
    - main
```

### With Caching

```yaml
lint:
  image: rust:latest
  cache:
    paths:
      - .cargo/
  before_script:
    - export CARGO_HOME="$(pwd)/.cargo"
    - cargo install maki
  script:
    - maki lint **/*.fsh
```

### Artifacts

```yaml
lint:
  image: rust:latest
  script:
    - maki lint --format json **/*.fsh > lint-report.json
  artifacts:
    reports:
      codequality: lint-report.json
    when: always
```

## Jenkins

### Declarative Pipeline

```groovy
pipeline {
    agent any
    
    stages {
        stage('Setup') {
            steps {
                sh 'cargo install maki'
            }
        }
        
        stage('Lint') {
            steps {
                sh 'maki lint **/*.fsh'
            }
        }
    }
    
    post {
        always {
            junit 'lint-report.xml'
        }
    }
}
```

## CircleCI

```yaml
version: 2.1

jobs:
  lint:
    docker:
      - image: cimg/rust:1.80
    steps:
      - checkout
      - restore_cache:
          keys:
            - cargo-cache-{{ checksum "Cargo.lock" }}
      - run:
          name: Install FSH Lint
          command: cargo install maki
      - save_cache:
          paths:
            - ~/.cargo
          key: cargo-cache-{{ checksum "Cargo.lock" }}
      - run:
          name: Lint FSH files
          command: maki lint **/*.fsh

workflows:
  version: 2
  lint:
    jobs:
      - lint
```

## Azure Pipelines

```yaml
trigger:
  - main

pool:
  vmImage: 'ubuntu-latest'

steps:
- task: RustInstaller@1
  inputs:
    rustVersion: 'stable'

- script: cargo install maki
  displayName: 'Install FSH Lint'

- script: maki lint **/*.fsh
  displayName: 'Lint FSH files'
```

## Pre-commit Hook

### Format and Lint

Install FSH Lint as a pre-commit hook to format and lint before commits:

```.pre-commit-config.yaml
repos:
  - repo: local
    hooks:
      - id: maki-format
        name: FSH Format
        entry: maki format
        language: system
        files: \.fsh$
      - id: maki-lint
        name: FSH Lint
        entry: maki lint --fix
        language: system
        files: \.fsh$
```

### Format Check Only

Or just check formatting without modifying files:

```.pre-commit-config.yaml
repos:
  - repo: local
    hooks:
      - id: maki-format-check
        name: FSH Format Check
        entry: maki format --check
        language: system
        files: \.fsh$
        pass_filenames: true
```

### Git Hook Script

Alternatively, use a custom git hook (`.git/hooks/pre-commit`):

```bash
#!/bin/bash

# Format FSH files
echo "Formatting FSH files..."
maki format **/*.fsh

# Check if formatting changed files
if ! git diff --quiet; then
  echo "Files were formatted. Please review changes and commit again."
  git add **/*.fsh
fi

# Lint FSH files
echo "Linting FSH files..."
maki lint --fix **/*.fsh

if [ $? -ne 0 ]; then
  echo "Linting failed. Please fix errors before committing."
  exit 1
fi
```

Make it executable:

```bash
chmod +x .git/hooks/pre-commit
```

## Best Practices

1. **Check Formatting First** - Run `maki format --check` before linting to catch style issues
2. **Cache Dependencies** - Cache Cargo registry for faster builds
3. **Auto-format and Auto-fix** - Apply formatting and safe fixes automatically in CI
4. **Fail on Errors** - Treat errors as build failures
5. **Report Artifacts** - Save lint reports as artifacts
6. **Separate Jobs** - Use separate jobs for formatting and linting for parallel execution
7. **Matrix Testing** - Test on multiple OS if needed

### Recommended Workflow

For best results, structure your workflow like this:

1. Format check (fast, catches style issues)
2. Linting (catches logical errors)
3. Auto-fix and commit (optional, for automation)

```yaml
jobs:
  format:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Check formatting
        run: maki format --check **/*.fsh

  lint:
    runs-on: ubuntu-latest
    needs: format
    steps:
      - uses: actions/checkout@v4
      - name: Lint FSH files
        run: maki lint **/*.fsh
```

## Troubleshooting

### Slow Installation

Use binary releases instead of building from source:

```bash
curl -L https://github.com/octofhir/maki/releases/latest/download/maki-linux.tar.gz | tar xz
sudo mv maki /usr/local/bin/
```

### Memory Issues

Limit parallel execution:

```bash
export CARGO_BUILD_JOBS=2
cargo install maki
```
