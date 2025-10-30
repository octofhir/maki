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
      
      - name: Lint FSH files
        run: maki lint **/*.fsh
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
            ~/.cargo/bin/maki
            ~/.cargo/registry
            ~/.cargo/git
          key: ${{ runner.os }}-cargo-maki
      
      - name: Install FSH Lint
        run: cargo install maki || true
      
      - name: Lint with fixes
        run: maki lint --fix **/*.fsh
      
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

Install FSH Lint as a pre-commit hook:

```.pre-commit-config.yaml
repos:
  - repo: local
    hooks:
      - id: maki
        name: FSH Lint
        entry: maki lint --fix
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
curl -L https://github.com/octofhir/maki-rs/releases/latest/download/maki-linux.tar.gz | tar xz
sudo mv maki /usr/local/bin/
```

### Memory Issues

Limit parallel execution:

```bash
export CARGO_BUILD_JOBS=2
cargo install maki
```
