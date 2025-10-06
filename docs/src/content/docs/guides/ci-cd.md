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
        run: cargo install fsh-lint
      
      - name: Lint FSH files
        run: fsh-lint lint **/*.fsh
```

### With Caching

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
      
      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/bin/fsh-lint
            ~/.cargo/registry
            ~/.cargo/git
          key: ${{ runner.os }}-cargo-fsh-lint
      
      - name: Install FSH Lint
        run: cargo install fsh-lint || true
      
      - name: Lint with fixes
        run: fsh-lint lint --fix **/*.fsh
      
      - name: Commit fixes
        uses: stefanzweifel/git-auto-commit-action@v5
        with:
          commit_message: "style: auto-fix FSH lint issues"
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
      - run: cargo install fsh-lint
      - run: fsh-lint lint **/*.fsh
```

## GitLab CI

### Basic Pipeline

```yaml
lint:
  image: rust:latest
  before_script:
    - cargo install fsh-lint
  script:
    - fsh-lint lint **/*.fsh
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
    - cargo install fsh-lint
  script:
    - fsh-lint lint **/*.fsh
```

### Artifacts

```yaml
lint:
  image: rust:latest
  script:
    - fsh-lint lint --format json **/*.fsh > lint-report.json
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
                sh 'cargo install fsh-lint'
            }
        }
        
        stage('Lint') {
            steps {
                sh 'fsh-lint lint **/*.fsh'
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
          command: cargo install fsh-lint
      - save_cache:
          paths:
            - ~/.cargo
          key: cargo-cache-{{ checksum "Cargo.lock" }}
      - run:
          name: Lint FSH files
          command: fsh-lint lint **/*.fsh

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

- script: cargo install fsh-lint
  displayName: 'Install FSH Lint'

- script: fsh-lint lint **/*.fsh
  displayName: 'Lint FSH files'
```

## Pre-commit Hook

Install FSH Lint as a pre-commit hook:

```.pre-commit-config.yaml
repos:
  - repo: local
    hooks:
      - id: fsh-lint
        name: FSH Lint
        entry: fsh-lint lint --fix
        language: system
        files: \.fsh$
```

## Best Practices

1. **Cache Dependencies** - Cache Cargo registry for faster builds
2. **Auto-fix in CI** - Apply safe fixes automatically
3. **Fail on Errors** - Treat errors as build failures
4. **Report Artifacts** - Save lint reports as artifacts
5. **Matrix Testing** - Test on multiple OS if needed

## Troubleshooting

### Slow Installation

Use binary releases instead of building from source:

```bash
curl -L https://github.com/octofhir/fsh-lint-rs/releases/latest/download/fsh-lint-linux.tar.gz | tar xz
sudo mv fsh-lint /usr/local/bin/
```

### Memory Issues

Limit parallel execution:

```bash
export CARGO_BUILD_JOBS=2
cargo install fsh-lint
```
