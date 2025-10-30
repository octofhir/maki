# maki-formatter

Formatter library for FHIR Shorthand (FSH) files.

## Overview

`maki-formatter` provides a clean API for formatting FSH files. It wraps the formatter functionality from `maki-core` and provides convenient access to formatting capabilities.

## Features

- **CST-based formatting**: Lossless formatting that preserves comments and whitespace
- **Configurable style**: Control indentation, line width, alignment, and more
- **Incremental formatting**: Format specific nodes or entire files
- **Diff output**: Show formatting changes before applying them

## Usage

```rust
use maki_formatter::{Formatter, FormatterConfiguration, FormatMode};

// Create formatter with default configuration
let config = FormatterConfiguration::default();
let formatter = Formatter::new(config);

// Format FSH source code
let source = r#"
Profile: MyPatient
Parent: Patient
* name 1..1
"#;

let result = formatter.format(source, FormatMode::File)?;
println!("{}", result.formatted_text);
```

## Configuration

Formatting behavior can be configured:

```rust
use maki_formatter::{FormatterConfiguration, CaretAlignment};

let config = FormatterConfiguration {
    indent_size: 2,
    line_width: 100,
    caret_alignment: CaretAlignment::Consecutive,
    ..Default::default()
};
```

## CLI Integration

The formatter is integrated with the MAKI CLI:

```bash
# Format files
maki fmt src/

# Check formatting without modifying files
maki fmt --check src/

# Show diff of proposed changes
maki fmt --diff src/
```

## Status

✅ **Available** - The formatter functionality is fully implemented in `maki-core` and wrapped by this crate.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
