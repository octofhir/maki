---
title: CLI Commands
description: MAKI command-line interface reference
---

Complete reference for all MAKI commands.

## `maki build`

Build FSH files to FHIR resources (SUSHI-compatible).

```bash
maki build [OPTIONS] [PROJECT_PATH]
```

The build command compiles FSH files from `input/fsh/` into FHIR JSON resources in `fsh-generated/`.

### Options

#### Build Options

- `--output, -o <PATH>` - Output directory (default: `fsh-generated`)
- `--snapshot` - Generate snapshots in StructureDefinitions
- `--preprocessed` - Output preprocessed FSH for debugging
- `--clean` - Clean output directory before building
- `--progress` - Show progress bar during build
- `--no-cache` - Disable incremental compilation cache
- `--skip-deps` - Skip installing FHIR package dependencies

#### Quality Options

- `--lint` - Run linter before build
- `--strict` - Treat warnings as errors (requires `--lint`)
- `--format` - Auto-format FSH files before build

#### Configuration

- `--config, -c <KEY:VALUE>` - Override config values (version, status, releaselabel)

### Examples

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

### Build Process

1. **Load Configuration** - Reads `sushi-config.yaml` or `maki.yaml`
2. **Format** (optional) - Auto-formats FSH files if `--format` is specified
3. **Lint** (optional) - Runs linter if `--lint` is specified
4. **Parse FSH** - Parses all FSH files from `input/fsh/`
5. **Build Semantic Model** - Constructs the semantic representation
6. **Export Resources** - Generates FHIR JSON for all resource types
7. **Generate Artifacts** - Creates `package.json`, FSH index, etc.

### Output Structure

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

---

## `maki gofsh`

Convert FHIR resources (JSON/XML) back to FSH (GoFSH functionality).

```bash
maki gofsh [OPTIONS] <INPUT>
```

### Options

- `--output, -o <PATH>` - Output directory for FSH files (default: `output`)
- `--fhir-version <VERSION>` - FHIR version: `R4` or `R5` (default: `R4`)
- `--dependency, -d <PKG>` - FHIR package dependencies (e.g., `hl7.fhir.us.core@5.0.1`)
- `--strategy <STRATEGY>` - File organization strategy:
  - `file` - One file per definition (default)
  - `type` - Group by FSH type (profiles.fsh, valuesets.fsh, etc.)
  - `profile` - Group by profile
  - `single` - All definitions in one file
- `--indent-size <N>` - Number of spaces for indentation (default: 2)
- `--line-width <N>` - Maximum line width (default: 100)
- `--progress` - Show progress bar and detailed output

### Examples

```bash
# Convert FHIR resources in a directory
maki gofsh ./fsh-generated

# Specify output directory
maki gofsh ./fsh-generated -o ./input/fsh

# With FHIR dependencies
maki gofsh ./resources -d hl7.fhir.us.core@5.0.1

# Use R5 FHIR version
maki gofsh ./resources --fhir-version R5

# With progress reporting
maki gofsh ./resources --progress

# Group output by FSH type
maki gofsh ./resources --strategy type

# All in one file
maki gofsh ./resources --strategy single
```

### Conversion Process

1. **Setup Packages** - Installs base FHIR packages and dependencies
2. **Load Resources** - Reads FHIR JSON/XML files from input
3. **Process Resources** - Extracts FSH definitions:
   - StructureDefinitions → Profiles, Extensions, Logical Models
   - ValueSets → ValueSet definitions
   - CodeSystems → CodeSystem definitions
   - Other resources → Instance definitions
4. **Optimize** - Applies rule optimizations:
   - Remove duplicate rules
   - Combine related rules
   - Simplify cardinality expressions
   - Remove implied/redundant rules
5. **Write FSH** - Generates formatted FSH files
6. **Generate Config** - Creates `sushi-config.yaml` and `.makirc.json`

### Output Structure

With `--strategy file` (default):
```
output/
├── MyProfile.fsh
├── MyExtension.fsh
├── MyValueSet.fsh
├── sushi-config.yaml
└── .makirc.json
```

With `--strategy type`:
```
output/
├── profiles.fsh
├── extensions.fsh
├── valuesets.fsh
├── codesystems.fsh
├── instances.fsh
├── sushi-config.yaml
└── .makirc.json
```

---

## `maki lint`

Lint FSH files and report diagnostics.

```bash
maki lint [OPTIONS] <FILES>...
```

### Options

#### Autofix Options

- `--fix` - Automatically fix safe issues (no semantic changes)
- `--unsafe` - Apply unsafe fixes (semantic changes) in addition to safe fixes
- `--dry-run` - Preview fixes without modifying files
- `--interactive`, `-i` - Prompt for confirmation on each unsafe fix
- `-w, --write` - Write fixes to files (alias for `--fix`)

#### Diagnostic Options

- `--severity <LEVEL>` - Only show diagnostics at or above this level: `error`, `warning`, `info`, `hint`
- `--format <FORMAT>` - Output format: `human`, `json`, `sarif`, `github`
- `--max-diagnostics <N>` - Limit number of diagnostics shown

#### Configuration Options

- `--config <PATH>` - Path to configuration file
- `--no-config` - Don't load configuration files

### Examples

```bash
# Lint all FSH files
maki lint **/*.fsh

# Apply safe fixes only
maki lint --fix input/fsh/*.fsh

# Apply all fixes (safe + unsafe)
maki lint --fix --unsafe input/fsh/*.fsh

# Preview fixes without applying
maki lint --fix --dry-run input/fsh/*.fsh

# Interactive mode - review each unsafe fix
maki lint --fix --unsafe --interactive input/fsh/*.fsh

# Show only errors
maki lint --severity error **/*.fsh

# Output JSON format
maki lint --format json **/*.fsh > diagnostics.json
```

### Fix Safety Levels

**Safe fixes** (applied with `--fix`):
- Add missing metadata (`Id`, `Title`, `Description`)
- Remove unused code (redundant aliases)
- Fix formatting and whitespace
- No semantic changes

**Unsafe fixes** (require `--unsafe` flag):
- Change naming conventions
- Add FHIR constraints
- Modify cardinality
- Semantic changes that should be reviewed

See the [Automatic Fixes guide](/guides/autofix/) for detailed information.

## `maki format`

Automatically format FSH files to maintain consistent code style.

```bash
maki format [OPTIONS] <FILES>...
```

The formatter uses a lossless Concrete Syntax Tree (CST) to ensure perfect preservation of:
- All comments (line and block)
- Blank lines and whitespace
- Semantic content

See the [FSH Formatter Guide](/maki/guides/formatter/) for detailed documentation.

### Options

- `--check` - Check if files are formatted without modifying them (exit code 1 if formatting needed)
- `--diff` - Show formatting differences without modifying files
- `--config <PATH>` - Path to configuration file
- `--no-config` - Don't load configuration files
- `-v, --verbose` - Enable verbose output

### Examples

```bash
# Format all FSH files
maki format **/*.fsh

# Format specific directory
maki format input/fsh/*.fsh

# Check formatting without modifying (useful for CI)
maki format --check **/*.fsh

# Show what would change
maki format --diff input/fsh/*.fsh

# Use custom configuration
maki format --config custom-config.json **/*.fsh
```

### Formatting Features

The formatter provides:

- **Consistent indentation** - Configurable spaces or tabs
- **Rule alignment** - Align cardinality and flags for readability
- **Spacing normalization** - Consistent spacing around `:` and `=`
- **Blank line control** - Maintain intentional spacing
- **Comment preservation** - All comments are preserved exactly

**Example:**

```fsh
// Before formatting
Profile:MyProfile
Parent:Patient
Id:  my-profile
* name 1..1 MS
* birthDate 1..1 MS
* gender 1..1 MS

// After formatting
Profile: MyProfile
Parent: Patient
Id: my-profile

* name      1..1 MS
* birthDate 1..1 MS
* gender    1..1 MS
```

### CI/CD Integration

Use `--check` mode in continuous integration:

```bash
# Exit code 0 if formatted, 1 if needs formatting
maki format --check input/fsh/**/*.fsh
```

Exit codes:
- `0` - All files are properly formatted
- `1` - Some files need formatting
- `2` - Error occurred during formatting

### Performance

The formatter is highly optimized:

- Single files: <50ms
- Large projects: Parallel processing enabled automatically
- Memory efficient: Streaming processing
- Token optimization: 2-5% performance boost

See the [Formatter Guide](/maki/guides/formatter/) for configuration options and best practices.

## `maki init`

Initialize a new FHIR Implementation Guide project.

```bash
maki init [OPTIONS] [NAME]
```

### Arguments

- `NAME` - Project name (default: `MyIG`)

### Options

- `--default` - Use default values without prompting (non-interactive mode)

### Examples

```bash
# Initialize interactively
maki init

# Initialize with project name
maki init MyCustomIG

# Non-interactive with defaults
maki init --default
```

### Generated Files

The init command creates the basic IG structure:

```
my-ig/
├── input/
│   └── fsh/
│       └── patient.fsh (example)
├── sushi-config.yaml
└── .makirc.json
```

## `maki rules`

List available rules.

```bash
maki rules [OPTIONS]
```

### Options

- `--detailed` - Show detailed information
- `--category <CATEGORY>` - Filter by category
- `--search <QUERY>` - Search rules

### Examples

```bash
# List all rules
maki rules

# Show detailed info for a category
maki rules --detailed --category style

# Search for specific rules
maki rules --search naming
```

## `maki check`

Check configuration validity.

```bash
maki check [OPTIONS]
```

### Options

- `--config <PATH>` - Path to configuration file

### Examples

```bash
# Check default config
maki check

# Check specific config
maki check --config custom-config.json
```

## Global Options

Available for all commands:

- `-h, --help` - Print help information
- `-V, --version` - Print version information
- `-v, --verbose` - Enable verbose output
- `--color <WHEN>` - Colorize output: `auto`, `always`, `never`

## Exit Codes

See [Exit Codes](/cli/exit-codes/) for details.
