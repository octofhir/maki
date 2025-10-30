# FSH Lint Examples

This directory contains example FSH files demonstrating various linting scenarios, along with configuration templates.

## Example Files

### ✅ Good Examples

- **[patient-profile.fsh](patient-profile.fsh)** - Well-formed profile with complete metadata, proper constraints, and good practices

### ⚠️ Warning Examples

- **[missing-metadata.fsh](missing-metadata.fsh)** - Profiles and extensions missing recommended metadata fields
- **[naming-issues.fsh](naming-issues.fsh)** - Naming convention violations (name/id/title/filename mismatches)

### ❌ Error Examples

- **[invalid-cardinality.fsh](invalid-cardinality.fsh)** - Various cardinality constraint errors
- **[extension-issues.fsh](extension-issues.fsh)** - Extension definition problems (missing context, conflicting constraints)
- **[valueset-examples.fsh](valueset-examples.fsh)** - ValueSet definition issues
- **[binding-strength-issues.fsh](binding-strength-issues.fsh)** - Missing or invalid binding strength specifications

## Configuration Templates

### [configs/default.makirc](configs/default.makirc)

Balanced configuration suitable for most projects:
- All correctness rules as errors
- Naming and style rules as warnings
- Safe autofixes enabled by default

**Use when:** Starting a new project or maintaining existing FSH code

```bash
maki --config examples/configs/default.makirc your-file.fsh
```

### [configs/strict.makirc](configs/strict.makirc)

Strict enforcement with all rules as errors:
- All naming conventions must be followed exactly
- Complete metadata required for all definitions
- Maximum code quality and consistency

**Use when:** Publishing official FHIR implementation guides or maintaining critical healthcare systems

```bash
maki --config examples/configs/strict.makirc your-file.fsh
```

### [configs/minimal.makirc](configs/minimal.makirc)

Minimal configuration with only critical correctness rules:
- Required fields must be present
- Invalid cardinality caught
- Binding strength required
- Extension context must be specified

**Use when:** Rapid prototyping, early development, or migrating legacy code

```bash
maki --config examples/configs/minimal.makirc your-file.fsh
```

## Testing the Examples

### Lint All Examples

```bash
just test-examples
# or
maki examples/*.fsh
```

### Test with Different Configs

```bash
just test-configs
# or manually:
maki --config examples/configs/default.makirc examples/*.fsh
maki --config examples/configs/strict.makirc examples/*.fsh
maki --config examples/configs/minimal.makirc examples/*.fsh
```

### Apply Safe Fixes

```bash
# Dry run to see what would be fixed
maki fix --dry-run examples/invalid-cardinality.fsh

# Apply safe fixes only
maki fix examples/invalid-cardinality.fsh

# Apply all fixes including unsafe (with confirmation)
maki fix --unsafe examples/invalid-cardinality.fsh

# Interactive mode - confirm each unsafe fix
maki fix --interactive examples/invalid-cardinality.fsh
```

### Output Formats

```bash
# Human-readable (default)
maki examples/patient-profile.fsh

# JSON output
maki --format json examples/patient-profile.fsh

# SARIF output for CI/CD integration
maki --format sarif examples/patient-profile.fsh > results.sarif
```

## Expected Diagnostics

### patient-profile.fsh
✅ **Expected:** No errors or warnings (well-formed example)

### invalid-cardinality.fsh
❌ **Expected errors:**
- Line 8: Upper bound (0) less than lower bound (1)
- Line 14: Invalid cardinality syntax
- Line 20: Cardinality conflicts with parent
- Line 26: Non-numeric cardinality values

⚠️ **Expected warnings:**
- Line 11: Redundant cardinality (same as parent)
- Line 17: Narrowing cardinality breaks conformance
- Line 23: Upper bound exceeds parent's limit

### missing-metadata.fsh
⚠️ **Expected warnings:**
- Missing Id, Title, Description
- Missing version, status, publisher
- Missing documentation for extensions

### naming-issues.fsh
❌ **Expected errors:**
- Profile name doesn't match Id (kebab-case mismatch)
- CodeSystem name doesn't match conventions
- ValueSet name includes _VS in Id (should be removed)

### extension-issues.fsh
❌ **Expected errors:**
- Missing Id and Title
- Missing context specification
- Extension has both value[x] and sub-extensions
- Conflicting type constraints on value[x]

### valueset-examples.fsh
❌ **Expected errors:**
- Missing Id
- Invalid Parent keyword (ValueSets don't have parents)
- Invalid include syntax
- Invalid URL formats

⚠️ **Expected warnings:**
- Duplicate include statements
- Empty ValueSet with no content

### binding-strength-issues.fsh
❌ **Expected errors:**
- Missing binding strength specifications
- Invalid binding strength values

## Rule Coverage

These examples cover all rule categories:

| Category | Rules Demonstrated | Files |
|----------|-------------------|-------|
| **Blocking** | required-field-present | missing-metadata.fsh |
| **Naming** | *-name-matches-{id,title,filename} | naming-issues.fsh |
| **Metadata** | profile-assignment-present, missing-metadata | missing-metadata.fsh |
| **Binding** | binding-strength-present | binding-strength-issues.fsh |
| **Cardinality** | invalid-cardinality, redundant-cardinality | invalid-cardinality.fsh |
| **Extension** | extension-context-missing, conflicting-constraints | extension-issues.fsh |
| **ValueSet** | empty-valueset, invalid-syntax | valueset-examples.fsh |
| **Style** | naming conventions, title case | All files |

## Adding New Examples

When adding new example files:

1. **Name clearly:** Use descriptive names indicating the issue demonstrated
2. **Comment extensively:** Mark each issue with ERROR, WARNING, or INFO comments
3. **Include good examples:** Show the correct way alongside errors
4. **Update this README:** Add entry in the table above
5. **Add golden tests:** Create corresponding test in `crates/maki-core/tests/golden_files/`

## Using Examples in Tests

These examples are used in golden file tests:

```rust
#[test]
fn test_invalid_cardinality_diagnostics() {
    let source = include_str!("../examples/invalid-cardinality.fsh");
    let diagnostics = lint_source(source);

    insta::assert_yaml_snapshot!(diagnostics);
}
```

See [`crates/maki-core/tests/golden_file_tests.rs`](../crates/maki-core/tests/golden_file_tests.rs) for implementation.
