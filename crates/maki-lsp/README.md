# maki-lsp

Language Server Protocol (LSP) implementation for FHIR Shorthand (FSH).

## Overview

`maki-lsp` provides IDE support for FHIR Shorthand files, enabling features like:

- **Diagnostics**: Real-time syntax and semantic error detection
- **Code Completion**: Intelligent autocompletion for FSH elements
- **Go-to-Definition**: Navigate to resource definitions
- **Hover Information**: Show documentation and type information
- **Code Actions**: Quick fixes and refactoring suggestions
- **Document Formatting**: Format FSH files on save

## Status

⚠️ **Under Development** - This crate is currently a stub and will be fully implemented in future tasks.

## Future Implementation

This LSP server will be implemented according to the MAKI roadmap:
- Task 31: LSP Server implementation
- Task 32: IDE integration
- Task 33: Advanced LSP features

## Integration

The LSP server is designed to work with:
- VS Code (via extension)
- Neovim (via native LSP)
- Any LSP-compatible editor

## Usage

```bash
# Start the LSP server (future)
maki lsp
```

## Architecture

The LSP server leverages:
- `maki-core` for parsing and semantic analysis
- `maki-rules` for diagnostics
- `tower-lsp` for LSP protocol implementation

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
