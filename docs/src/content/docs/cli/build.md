---
title: maki build
description: Build FSH files to FHIR resources (SUSHI-compatible)
---

Build FSH files to FHIR resources. This command is a drop-in replacement for SUSHI.

```bash
maki build [OPTIONS] [PROJECT_PATH]
```

The build command compiles FSH files from `input/fsh/` into FHIR JSON resources in `fsh-generated/`.

## Options

### Build Options

- `--output, -o <PATH>` - Output directory (default: `fsh-generated`)
- `--snapshot` - Generate snapshots in StructureDefinitions
- `--preprocessed` - Output preprocessed FSH for debugging
- `--clean` - Clean output directory before building
- `--progress` - Show progress bar during build
- `--no-cache` - Disable incremental compilation cache
- `--skip-deps` - Skip installing FHIR package dependencies

### Quality Options

- `--lint` - Run linter before build
- `--strict` - Treat warnings as errors (requires `--lint`)
- `--format` - Auto-format FSH files before build

### Configuration

- `--config, -c <KEY:VALUE>` - Override config values (version, status, releaselabel)

## Examples

```bash
# Build current directory
maki build

# Build with progress bar
maki build --progress

# Run linter before building
maki build --lint

# Format FSH files before building
maki build --format

# Clean output and rebuild
maki build --clean

# Strict mode (treat warnings as errors)
maki build --lint --strict

# Specify project path and output
maki build ./my-ig --output ./output

# Override version for release
maki build -c version:1.0.0 -c status:active
```

## Build Process

1. **Load Configuration** - Reads `sushi-config.yaml` or `maki.yaml`
2. **Format** (optional) - Auto-formats FSH files if `--format` is specified
3. **Lint** (optional) - Runs linter if `--lint` is specified
4. **Parse FSH** - Parses all FSH files from `input/fsh/`
5. **Build Semantic Model** - Constructs the semantic representation
6. **Export Resources** - Generates FHIR JSON for all resource types
7. **Generate Artifacts** - Creates `package.json`, FSH index, etc.

## Output Structure

```
fsh-generated/
├── resources/
│   ├── StructureDefinition-*.json
│   ├── ValueSet-*.json
│   ├── CodeSystem-*.json
│   └── *.json (instances)
├── package.json
└── fsh-index.json
```

## SUSHI Compatibility

MAKI build is designed as a drop-in replacement for SUSHI:

- **Same input structure** - Reads from `input/fsh/` directory
- **Same output structure** - Generates to `fsh-generated/` directory
- **Same configuration** - Uses `sushi-config.yaml`
- **Same resource format** - Produces identical FHIR JSON output

### Migrating from SUSHI

Simply replace your `sushi` command with `maki build`:

```bash
# Before (SUSHI)
sushi .

# After (MAKI)
maki build
```

## Performance

MAKI build is significantly faster than SUSHI:

- **Parallel processing** - Utilizes all CPU cores
- **Incremental builds** - Only recompiles changed files
- **Optimized parsing** - Rust-based FSH parser
- **Efficient export** - Streaming JSON generation

Typical performance improvements:

| Project Size | SUSHI | MAKI |
|-------------|-------|------|
| Small (10 files) | 5s | <1s |
| Medium (100 files) | 30s | 3s |
| Large (500+ files) | 2min+ | 15s |
