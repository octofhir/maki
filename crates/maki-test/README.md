# maki-test

Testing framework for FHIR Shorthand (FSH) files.

## Overview

`maki-test` provides a testing framework for validating FSH resources and Implementation Guides. It enables test-driven development for FHIR artifacts.

## Status

⚠️ **Under Development** - This crate is currently a stub and will be fully implemented in future tasks.

## Future Features

The testing framework will provide:

- **FSH Test Suites**: Define test cases in FSH or JSON
- **Instance Validation**: Validate FSH instances against profiles
- **Expected Output Comparison**: Compare generated FHIR with expected output
- **Integration Testing**: Test complete IG builds
- **Snapshot Testing**: Capture and verify FHIR output
- **Test Discovery**: Automatic test discovery in projects

## Usage (Future)

```bash
# Run all tests in a project
maki test

# Run specific test files
maki test tests/patient-tests.fsh

# Watch mode for TDD
maki test --watch
```

## Test File Format (Future)

```fsh
// Test file: tests/patient-validation.fsh

Instance: TestPatient
InstanceOf: MyPatientProfile
* name.given = "John"
* name.family = "Doe"
* birthDate = "1980-01-01"

// Expected to validate successfully
* ^test:expect = "valid"

Instance: InvalidPatient
InstanceOf: MyPatientProfile
// Missing required name field
* birthDate = "1980-01-01"

// Expected to fail validation
* ^test:expect = "error"
* ^test:errorCode = "required-field-missing"
```

## Future Implementation

This testing framework will be implemented in:
- Task 34: Test Runner implementation
- Task 35: Test format specification
- Task 36: Integration with CI/CD

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
