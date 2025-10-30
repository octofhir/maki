# Parser Layer Tests

This directory contains comprehensive tests for the FSH parser layer, covering all requirements specified in task 4.3.

## Test Coverage

### 1. Valid FSH File Parsing Tests
- **Simple Profile Parsing**: Tests parsing of basic FSH profile definitions
- **Extension Definition Parsing**: Tests parsing of FSH extension definitions  
- **Multiple Resources Parsing**: Tests parsing files with multiple FSH resources
- **Large Content Parsing**: Tests parser performance with large FSH files
- **Unicode Content Parsing**: Tests parsing of FSH files with Unicode characters

### 2. Invalid FSH File Parsing Tests
- **Syntax Error Handling**: Tests parser behavior with syntax errors (missing colons, etc.)
- **Incomplete Object Parsing**: Tests parsing of incomplete or malformed FSH structures
- **Mixed Valid/Invalid Content**: Tests parser recovery when valid and invalid content is mixed

### 3. Parse Error Recovery and Reporting Tests
- **Error Location Reporting**: Verifies that parse errors include accurate line/column information
- **Error Message Quality**: Tests that error messages are descriptive and helpful
- **Continued Processing**: Tests that parser continues processing after encountering errors
- **Error Recovery**: Tests parser's ability to recover from errors and parse subsequent valid content

### 4. Caching Behavior Tests
- **Hash-based Cache Keys**: Tests that content hashing works correctly for cache keys
- **Cache Hit/Miss Behavior**: Verifies cache returns same results for identical content
- **Cache Invalidation**: Tests cache invalidation when content changes
- **Cache Capacity Limits**: Tests LRU eviction when cache reaches capacity
- **Incremental Parsing Cache**: Tests caching behavior with incremental parsing
- **Language Change Cache Clearing**: Tests that cache is cleared when parser language changes

### 5. Performance and Concurrency Tests
- **Concurrent Parsing**: Tests thread safety of parser instances
- **Cached Parser Concurrency**: Tests thread safety of cached parser with shared cache
- **Large File Performance**: Tests parser performance with large FSH files

### 6. Integration Tests
- **File I/O Integration**: Tests parsing FSH content loaded from files
- **Parser Trait Implementation**: Tests that cached parser correctly implements Parser trait
- **Error Handling Integration**: Tests end-to-end error handling from parsing to reporting

## Test Architecture

### Tree-sitter Integration
The tests exercise the real `tree-sitter-fsh` grammar through lightweight
helpers (`mock_tree_sitter_fsh.rs`) that:
- Expose the grammar in a test-friendly wrapper
- Provide utilities for inspecting error nodes and building diagnostics
- Make it easy to simulate error scenarios and recovery logic

### Test Data
Test data is organized into:
- **Valid samples**: Well-formed FSH definitions (profiles, extensions, instances)
- **Invalid samples**: Purposefully malformed FSH constructs to test error handling
- **Edge cases**: Empty content, whitespace-only content, Unicode content

### Coverage Areas
The tests comprehensively cover:
- ✅ Parsing valid and invalid FSH files (Requirements 1.2, 12.2)
- ✅ Parse error recovery and reporting (Requirements 1.2, 12.2)  
- ✅ Caching behavior with hash-based keys (Requirements 8.4, 12.2)
- ✅ Performance with large files and concurrent access
- ✅ Integration with file I/O and error handling systems

## Running Tests

```bash
# Run all parser tests
cargo test --test parser_tests

# Run specific test
cargo test --test parser_tests test_parse_valid_simple_profile

# Run with output
cargo test --test parser_tests -- --nocapture
```

## Test Results

All 38 tests pass successfully, covering:
- 15 core parsing functionality tests
- 8 caching behavior tests  
- 6 error handling and recovery tests
- 5 performance and concurrency tests
- 4 integration tests

The test suite validates that the parser layer meets all specified requirements and handles edge cases gracefully.
