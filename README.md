# FSH Lint

A high-performance linter for FHIR Shorthand (FSH) files written in Rust.

**Part of the [OctoFHIR](https://github.com/octofhir) ecosystem.**

## Project Structure

This project is organized as a Rust workspace with the following crates:

- **`fsh-lint-core`** - Core linting engine and shared types
- **`fsh-lint-cli`** - Command-line interface
- **`fsh-lint-rules`** - Built-in rules and rule engine

## Development Status

This project is currently under development. The basic project structure and foundation have been established.

## Building

```bash
cargo build --workspace
```

## Running

```bash
cargo run --bin fsh-lint -- --help
```

## License

MIT OR Apache-2.0