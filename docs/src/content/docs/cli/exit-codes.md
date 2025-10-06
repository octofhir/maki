---
title: Exit Codes
description: FSH Lint exit code reference
---

FSH Lint uses specific exit codes to indicate different outcomes.

## Exit Code Reference

### `0` - Success

No errors found. May contain warnings or hints.

```bash
fsh-lint lint file.fsh
echo $?  # 0
```

### `1` - Lint Errors

Lint errors were found in the files.

```bash
fsh-lint lint invalid.fsh
echo $?  # 1
```

### `2` - Invalid Configuration

Configuration file is invalid or malformed.

```bash
fsh-lint lint --config invalid.json *.fsh
echo $?  # 2
```

### `3` - File Not Found

Specified files or directories not found.

```bash
fsh-lint lint nonexistent.fsh
echo $?  # 3
```

### `4` - Invalid Arguments

Invalid command-line arguments provided.

```bash
fsh-lint lint --unknown-flag *.fsh
echo $?  # 4
```

### `5` - Internal Error

An internal error occurred.

```bash
# Should not happen in normal use
echo $?  # 5
```

## CI/CD Integration

Use exit codes in CI/CD pipelines:

### GitHub Actions

```yaml
- name: Lint FSH files
  run: |
    fsh-lint lint **/*.fsh
    if [ $? -eq 1 ]; then
      echo "Lint errors found"
      exit 1
    fi
```

### GitLab CI

```yaml
lint:
  script:
    - fsh-lint lint **/*.fsh
  allow_failure: false
```

### Jenkins

```groovy
stage('Lint') {
  steps {
    sh '''
      fsh-lint lint **/*.fsh
      EXIT_CODE=$?
      if [ $EXIT_CODE -ne 0 ]; then
        echo "Linting failed with code $EXIT_CODE"
        exit $EXIT_CODE
      fi
    '''
  }
}
```

## Warning-Only Mode

Treat warnings as errors for stricter CI:

```bash
fsh-lint lint --severity warn **/*.fsh
```

This will exit with code `1` if any warnings are found.

## Ignoring Specific Exit Codes

Continue on warnings but fail on errors:

```bash
fsh-lint lint **/*.fsh || [ $? -eq 0 -o $? -eq 1 ] && echo "OK"
```
