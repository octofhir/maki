---
title: CI/CD Integration
description: Integrate MAKI with your CI/CD pipeline
---

Integrate MAKI into your continuous integration and deployment workflows.

## GitHub Actions

### Complete Build Workflow

```yaml
name: Build FHIR IG

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Download MAKI
        run: |
          curl -L https://github.com/octofhir/maki/releases/latest/download/maki-linux-x64 -o maki
          chmod +x maki
          sudo mv maki /usr/local/bin/

      - name: Check formatting
        run: maki fmt --check input/fsh/

      - name: Lint FSH files
        run: maki lint input/fsh/

      - name: Build IG
        run: maki build --progress

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: fsh-generated
          path: fsh-generated/
```

### Basic Lint Workflow

```yaml
name: Lint FSH Files

on: [push, pull_request]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Download MAKI
        run: |
          curl -L https://github.com/octofhir/maki/releases/latest/download/maki-linux-x64 -o maki
          chmod +x maki
          sudo mv maki /usr/local/bin/

      - name: Check formatting
        run: maki fmt --check input/fsh/

      - name: Lint FSH files
        run: maki lint input/fsh/
```

### Strict Build (CI Mode)

Build with strict mode - treat warnings as errors:

```yaml
name: Strict Build

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Download MAKI
        run: |
          curl -L https://github.com/octofhir/maki/releases/latest/download/maki-linux-x64 -o maki
          chmod +x maki
          sudo mv maki /usr/local/bin/

      - name: Build with strict mode
        run: maki build --lint --strict --progress
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

      - name: Download MAKI
        run: |
          curl -L https://github.com/octofhir/maki/releases/latest/download/maki-linux-x64 -o maki
          chmod +x maki
          sudo mv maki /usr/local/bin/

      - name: Format FSH files
        run: maki fmt --write input/fsh/

      - name: Lint with fixes
        run: maki lint --write input/fsh/

      - name: Commit changes
        uses: stefanzweifel/git-auto-commit-action@v5
        with:
          commit_message: "style: auto-format and fix FSH issues"
```

## GitLab CI

### Basic Pipeline

```yaml
build:
  image: ubuntu:latest
  before_script:
    - apt-get update && apt-get install -y curl
    - curl -L https://github.com/octofhir/maki/releases/latest/download/maki-linux-x64 -o /usr/local/bin/maki
    - chmod +x /usr/local/bin/maki
  script:
    - maki lint input/fsh/
    - maki build --progress
  only:
    - merge_requests
    - main
```

### With Artifacts

```yaml
build:
  image: ubuntu:latest
  before_script:
    - apt-get update && apt-get install -y curl
    - curl -L https://github.com/octofhir/maki/releases/latest/download/maki-linux-x64 -o /usr/local/bin/maki
    - chmod +x /usr/local/bin/maki
  script:
    - maki lint --format json input/fsh/ > lint-report.json || true
    - maki build --progress
  artifacts:
    paths:
      - fsh-generated/
      - lint-report.json
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
                sh '''
                    curl -L https://github.com/octofhir/maki/releases/latest/download/maki-linux-x64 -o maki
                    chmod +x maki
                    mv maki /usr/local/bin/
                '''
            }
        }

        stage('Lint') {
            steps {
                sh 'maki lint input/fsh/'
            }
        }

        stage('Build') {
            steps {
                sh 'maki build --progress'
            }
        }
    }

    post {
        always {
            archiveArtifacts artifacts: 'fsh-generated/**/*', allowEmptyArchive: true
        }
    }
}
```

## Azure Pipelines

```yaml
trigger:
  - main

pool:
  vmImage: 'ubuntu-latest'

steps:
- script: |
    curl -L https://github.com/octofhir/maki/releases/latest/download/maki-linux-x64 -o maki
    chmod +x maki
    sudo mv maki /usr/local/bin/
  displayName: 'Install MAKI'

- script: maki lint input/fsh/
  displayName: 'Lint FSH files'

- script: maki build --progress
  displayName: 'Build IG'

- task: PublishBuildArtifacts@1
  inputs:
    pathtoPublish: 'fsh-generated'
    artifactName: 'fsh-generated'
```

## Pre-commit Hook

### Format and Lint

Install MAKI as a pre-commit hook to format and lint before commits:

```.pre-commit-config.yaml
repos:
  - repo: local
    hooks:
      - id: maki-format
        name: FSH Format
        entry: maki fmt
        language: system
        files: \.fsh$
      - id: maki-lint
        name: FSH Lint
        entry: maki lint --write
        language: system
        files: \.fsh$
```

### Git Hook Script

Alternatively, use a custom git hook (`.git/hooks/pre-commit`):

```bash
#!/bin/bash

# Format FSH files
echo "Formatting FSH files..."
maki fmt input/fsh/

# Check if formatting changed files
if ! git diff --quiet; then
  echo "Files were formatted. Please review changes and commit again."
  git add input/fsh/*.fsh
fi

# Lint FSH files
echo "Linting FSH files..."
maki lint input/fsh/

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

1. **Use Pre-built Binaries** - Download binary releases for faster CI setup
2. **Check Formatting First** - Run `maki fmt --check` before linting
3. **Build with Quality Checks** - Use `maki build --lint --format` for integrated workflow
4. **Strict Mode for CI** - Use `--strict` to treat warnings as errors
5. **Save Artifacts** - Archive `fsh-generated/` for downstream use
6. **Fail Fast** - Separate format check → lint → build for quick feedback

### Recommended Workflow

For best results, structure your workflow like this:

```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install MAKI
        run: |
          curl -L https://github.com/octofhir/maki/releases/latest/download/maki-linux-x64 -o maki
          chmod +x maki
          sudo mv maki /usr/local/bin/

      - name: Format Check
        run: maki fmt --check input/fsh/

      - name: Lint
        run: maki lint input/fsh/

      - name: Build
        run: maki build --progress

      - name: Upload Artifacts
        uses: actions/upload-artifact@v4
        with:
          name: fsh-generated
          path: fsh-generated/
```

## Troubleshooting

### Binary Download Issues

If the binary download fails, try with specific version:

```bash
VERSION=0.0.3
curl -L https://github.com/octofhir/maki/releases/download/v${VERSION}/maki-linux-x64 -o maki
```

### Permission Denied

Ensure the binary is executable:

```bash
chmod +x maki
```

### Build Timeouts

For large IGs, increase the timeout or use `--skip-deps` if packages are pre-installed:

```bash
maki build --progress --skip-deps
```
