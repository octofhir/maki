---
title: Exit Codes
description: FSH Lint exit code reference
---

FSH Lint uses specific exit codes to indicate different outcomes.

## Exit Code Reference

### `0` - Success

No errors found. May contain warnings or hints.

```bash
maki lint file.fsh
echo $?  # 0
```

### `1` - Lint Errors

Lint errors were found in the files.

```bash
maki lint invalid.fsh
echo $?  # 1
```

### `2` - Invalid Configuration

Configuration file is invalid or malformed.

```bash
maki lint --config invalid.json *.fsh
echo $?  # 2
```

### `3` - File Not Found

Specified files or directories not found.

```bash
maki lint nonexistent.fsh
echo $?  # 3
```

### `4` - Invalid Arguments

Invalid command-line arguments provided.

```bash
maki lint --unknown-flag *.fsh
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
    maki lint **/*.fsh
    if [ $? -eq 1 ]; then
      echo "Lint errors found"
      exit 1
    fi
```

### GitLab CI

```yaml
lint:
  script:
    - maki lint **/*.fsh
  allow_failure: false
```

### Jenkins

```groovy
stage('Lint') {
  steps {
    sh '''
      maki lint **/*.fsh
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
maki lint --severity warn **/*.fsh
```

This will exit with code `1` if any warnings are found.

## Ignoring Specific Exit Codes

Continue on warnings but fail on errors:

```bash
maki lint **/*.fsh || [ $? -eq 0 -o $? -eq 1 ] && echo "OK"
```
