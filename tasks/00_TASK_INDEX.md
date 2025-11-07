# MAKI Task Index

This document provides an index of all implementation tasks for the MAKI project.

**Project Goal**: Build a complete SUSHI replacement in Rust that is 10-100x faster with 100% backward compatibility plus enhanced features (lint, format, LSP, test, docs).

**Total Timeline**: 32 weeks (8 months) - Updated with Phase 1.5 (GritQL Integration)
**Current Status**: Planning Complete → Ready for Implementation

---

## Task Numbering Convention

Tasks are numbered by phase and sequence:
- `0X` - Project setup and infrastructure
- `1X` - Phase 1: Core Compiler (Weeks 1-8)
- `1.5X` - Phase 1.5: GritQL Integration (Weeks 9-14)
- `2X` - Phase 2: Enhanced Linter (Weeks 15-18)
- `3X` - Phase 3: Formatter (Weeks 19-20)
- `4X` - Phase 4: LSP (Weeks 21-24)
- `5X` - Phase 5: Testing Framework (Weeks 25-27)
- `6X` - Phase 6: Documentation Generator (Weeks 28-30)
- `7X` - Phase 7: Migration Tools (Weeks 31-32)

---

## Phase 0: Project Setup & Infrastructure

**Status**: Partially Complete

| Task ID | Title | Status | Dependencies | Est. Time |
|---------|-------|--------|--------------|-----------|
| [00](./00_TASK_INDEX.md) | Task Index (this file) | ✅ Complete | None | N/A |
| [01](./01_repo_restructure.md) | Repository Restructuring | ✅ Complete | None | 2 days |
| [02](./02_ci_cd_setup.md) | CI/CD Pipeline Setup | 📝 Planned | 01 | 2 days |
| [03](./03_test_harness_setup.md) | SUSHI Comparison Test Harness | 📝 Planned | 01 | 2 days |

---

## Phase 1: Core Compiler (Weeks 1-8)

**Goal**: Drop-in SUSHI replacement for basic use cases

**Acceptance Criteria**:
- ✅ Compiles FSH → FHIR JSON resources
- ✅ Generates IG structure compatible with IG Publisher
- ✅ Passes SUSHI's 104 test cases
- ✅ Successfully compiles 3+ real-world IGs (US Core, IPS, mCODE)

### Week 1-2: FHIR Definitions Loader

| Task ID | Title | Status | Dependencies | Est. Time |
|---------|-------|--------|--------------|-----------|
| [10](./10_fhir_package_loader.md) | FHIR Package Loader | 📝 Planned | 01 | 3 days |
| [11](./11_fishable_trait.md) | Fishable Trait & Indexes | 📝 Planned | 10 | 2 days |
| [12](./12_version_support.md) | Multi-Version FHIR Support | 📝 Planned | 10, 11 | 2 days |

### Week 3-4: Semantic Analyzer

| Task ID | Title | Status | Dependencies | Est. Time |
|---------|-------|--------|--------------|-----------|
| [13](./13_symbol_table.md) | Enhanced Symbol Table | 📝 Planned | 11 | 2 days |
| [14](./14_path_resolver.md) | Path Resolution Algorithm | 📝 Planned | 11, 13 | 3 days |
| [15](./15_dependency_graph.md) | Dependency Graph | 📝 Planned | 13 | 2 days |
| [16](./16_ruleset_expansion.md) | RuleSet Parameter Expansion | 📝 Planned | 13 | 2 days |

### Week 5-6: Compiler - Structure Definitions

| Task ID | Title | Status | Dependencies | Est. Time |
|---------|-------|--------|--------------|-----------|
| [17](./17_profile_exporter.md) | Profile Exporter | 📝 Planned | 14, 15 | 4 days |
| [18](./18_extension_exporter.md) | Extension Exporter | 📝 Planned | 14, 15 | 2 days |
| [19](./19_logical_resource_exporter.md) | Logical/Resource Exporter | 📝 Planned | 14, 15 | 2 days |
| [20](./20_differential_snapshot.md) | Differential & Snapshot Generation | 📝 Planned | 17, 18, 19 | 3 days |

### Week 5-6: Compiler - Instances & Terminology

| Task ID | Title | Status | Dependencies | Est. Time |
|---------|-------|--------|--------------|-----------|
| [21](./21_instance_exporter.md) | Instance Exporter | 📝 Planned | 14, 15 | 4 days |
| [22](./22_valueset_exporter.md) | ValueSet Exporter | 📝 Planned | 15 | 2 days |
| [23](./23_codesystem_exporter.md) | CodeSystem Exporter | 📝 Planned | 15 | 2 days |

### Week 7-8: IG Generator

| Task ID | Title | Status | Dependencies | Est. Time |
|---------|-------|--------|--------------|-----------|
| [24](./24_config_parser.md) | sushi-config.yaml Parser | 📝 Planned | 01 | 2 days |
| [25](./25_ig_resource_generator.md) | ImplementationGuide Resource Generator | 📝 Planned | 24 | 3 days |
| [26](./26_file_structure_generator.md) | File Structure & Package Generator | 📝 Planned | 24, 25 | 2 days |
| [27](./27_predefined_resources.md) | Predefined Resource Loader | 📝 Planned | 24 | 2 days |

### Week 8: Integration & Validation

| Task ID | Title | Status | Dependencies | Est. Time |
|---------|-------|--------|--------------|-----------|
| [28](./28_build_command.md) | Build Command Integration | 📝 Planned | 17-27 | 2 days |

---

## Phase 1.5: GritQL Integration (Weeks 9-14)

**Goal**: Enable custom FSH linting rules without Rust knowledge via GritQL pattern language

**Acceptance Criteria**:
- ✅ GritQL execution engine complete with variable capture, predicates, and built-in functions
- ✅ GritQL code rewriting system for automatic autofix generation
- ✅ Both Rust and GritQL approaches available for all lint rules
- ✅ User can write custom rules in GritQL and contribute to repository
- ✅ Pattern validation and testing framework

### Phase 1.5 Core Modules

| Task ID | Title | Status | Dependencies | Est. Time |
|---------|-------|--------|--------------|-----------|
| [29](./29_gritql_execution_engine.md) | GritQL Execution Engine | 📝 Planned | 01, 14 | 6-7 weeks |
| [29.5](./29.5_gritql_code_rewriting.md) | GritQL Code Rewriting & Autofixes | 📝 Planned | 29 | 4 weeks |

**Key Features**:
- Pattern matching with variable capture (`$profile`, `$extension`)
- Predicates (regex `<:`, contains, field access)
- Built-in functions (`is_profile()`, `has_comment()`, `is_kebab_case()`)
- Code rewriting functions (`capitalize()`, `to_kebab_case()`, `to_pascal_case()`)
- Safe vs unsafe fix classification
- Pattern cookbook with 20+ examples
- Interactive pattern debugger

---

## Phase 2: Enhanced Linter (Weeks 15-18)

**Goal**: 50+ lint rules with excellent error messages and autofixes

**Acceptance Criteria**:
- ✅ 50+ rules implemented across all categories
- ✅ All rules have tests
- ✅ Autofix engine working for 30+ rules
- ✅ Integration with build command

### Week 15-16: Core Lint Rules

| Task ID | Title | Status | Dependencies | Est. Time |
|---------|-------|--------|--------------|-----------|
| [30](./30_naming_convention_rules.md) | Naming Convention Rules | 📝 Planned | 14, 15, 29 | 2 days |
| [31](./31_metadata_requirements.md) | Metadata Requirements Validation | 📝 Planned | 14, 15, 29 | 2 days |
| [32](./32_cardinality_validation.md) | Cardinality Validation | 📝 Planned | 14, 15, 29 | 3 days |

### Week 17: Semantic Lint Rules

| Task ID | Title | Status | Dependencies | Est. Time |
|---------|-------|--------|--------------|-----------|
| [33](./33_binding_strength_rules.md) | Binding Strength Rules | 📝 Planned | 30-32, 29 | 2 days |
| [34](./34_required_fields_validation.md) | Required Fields Validation | 📝 Planned | 30-32, 29 | 3 days |
| [35](./35_duplicate_detection.md) | Duplicate Detection Rules | 📝 Planned | 30-32, 29 | 2 days |
| [36](./36_profile_consistency_rules.md) | Profile Consistency Rules | 📝 Planned | 30-32, 29 | 2 days |

### Week 18: Autofix Engine & Integration

| Task ID | Title | Status | Dependencies | Est. Time |
|---------|-------|--------|--------------|-----------|
| [37](./37_autofix_engine.md) | Autofix Engine | 📝 Planned | 33-36, 29.5 | 3 days |
| [37.5](./37.5_lint_command.md) | Lint Command Integration | 📝 Planned | 37 | 2 days |

---

## Phase 3: Formatter (Weeks 19-20)

**Goal**: Standardized FSH code formatting

**Acceptance Criteria**:
- ✅ Lossless formatting (preserves all comments and semantics)
- ✅ Configurable style options
- ✅ CLI integration (`maki format`)
- ✅ LSP integration (format-on-save)

**Note**: Formatter core is already implemented in `crates/maki-core/src/cst/formatter.rs`

| Task ID | Title | Status | Dependencies | Est. Time |
|---------|-------|--------|--------------|-----------|
| [38](./38_fsh_formatter.md) | FSH Formatter Enhancement | 📝 Planned | 01, 03 | 2 days |
| [39](./39_format_command.md) | Format Command Integration | 📝 Planned | 38 | 1 day |

---

## Phase 4: LSP (Weeks 21-24)

**Goal**: Real-time IDE support for FSH

**Acceptance Criteria**:
- ✅ VS Code extension published
- ✅ Real-time diagnostics (<100ms)
- ✅ Code completion working
- ✅ Go to definition, hover, references

### Week 21-22: LSP Core

| Task ID | Title | Status | Dependencies | Est. Time |
|---------|-------|--------|--------------|-----------|
| [40](./40_lsp_server_setup.md) | LSP Server Setup | 📝 Planned | 01 | 2 days |
| [41](./41_document_sync.md) | Document Synchronization | 📝 Planned | 40 | 2 days |
| [42](./42_workspace_management.md) | Workspace Management | 📝 Planned | 40, 41 | 2 days |

### Week 23-24: LSP Features

| Task ID | Title | Status | Dependencies | Est. Time |
|---------|-------|--------|--------------|-----------|
| [43](./43_diagnostics.md) | LSP Diagnostics | 📝 Planned | 40-42, 37.5 | 2 days |
| [44](./44_completion.md) | Code Completion | 📝 Planned | 40-42 | 3 days |
| [45](./45_hover_definition.md) | Hover & Go to Definition | 📝 Planned | 40-42 | 2 days |
| [46](./46_code_actions.md) | Code Actions & Refactoring | 📝 Planned | 40-42, 37 | 2 days |
| [47](./47_vscode_extension.md) | VS Code Extension | 📝 Planned | 40-46 | 2 days |

---

## Phase 5: Testing Framework (Weeks 25-27)

**Goal**: Validate instances against profiles

**Acceptance Criteria**:
- ✅ Test file format defined
- ✅ Test runner working
- ✅ Coverage reporting
- ✅ Watch mode

| Task ID | Title | Status | Dependencies | Est. Time |
|---------|-------|--------|--------------|-----------|
| [50](./50_test_framework_design.md) | Test Framework Design | 📝 Planned | 21 | 2 days |
| [51](./51_test_runner.md) | Test Runner | 📝 Planned | 50 | 4 days |
| [52](./52_test_cli.md) | Test CLI Integration | 📝 Planned | 51 | 2 days |

---

## Phase 6: Documentation Generator (Weeks 28-30)

**Goal**: Auto-generate beautiful documentation from FSH

**Acceptance Criteria**:
- ✅ Markdown generation working
- ✅ PlantUML diagrams generated
- ✅ Interactive HTML site
- ✅ Search functionality

| Task ID | Title | Status | Dependencies | Est. Time |
|---------|-------|--------|--------------|-----------|
| [60](./60_markdown_generator.md) | Markdown Generator | 📝 Planned | 17-23 | 3 days |
| [61](./61_diagram_generator.md) | Diagram Generator | 📝 Planned | 17-23 | 3 days |
| [62](./62_html_generator.md) | HTML Generator | 📝 Planned | 60, 61 | 3 days |

---

## Phase 7: Migration Tools (Weeks 31-32)

**Goal**: Safe refactoring and FHIR version upgrades

**Acceptance Criteria**:
- ✅ Rename entity working
- ✅ Extract/inline RuleSet working
- ✅ FHIR version migration analysis
- ✅ Auto-migration where possible

| Task ID | Title | Status | Dependencies | Est. Time |
|---------|-------|--------|--------------|-----------|
| [70](./70_refactoring_engine.md) | Refactoring Engine | 📝 Planned | 13, 15 | 4 days |
| [71](./71_version_migration.md) | FHIR Version Migration | 📝 Planned | 12, 70 | 4 days |

---

## Task Status Legend

- ✅ **Complete**: Task finished and accepted
- 🚧 **In Progress**: Currently being worked on
- 📝 **Planned**: Specified but not started
- ⏸️ **Blocked**: Waiting on dependencies
- ❌ **Cancelled**: No longer needed

---

## Critical Path

The critical path for Phase 1 (minimum viable SUSHI replacement):

```
01 → 10 → 11 → 13 → 14 → 17 → 20 → 28 → 29
     ↓         ↓    ↓    ↓
     12        15   21   24 → 25 → 26 ↗
```

**Estimated Critical Path Time**: 8 weeks

---

## Acceptance Criteria Template

Every task file must include:

```markdown
## Acceptance Criteria

### Compilation
- [ ] All code compiles without errors
- [ ] `cargo build --workspace` succeeds
- [ ] No compiler warnings

### Code Quality
- [ ] `cargo clippy --workspace` passes with no warnings
- [ ] `cargo fmt --check` passes
- [ ] All public APIs have documentation

### Testing
- [ ] Unit tests written for all new code
- [ ] All tests pass: `cargo test --workspace`
- [ ] Integration tests pass (if applicable)
- [ ] Test coverage ≥80% for new code

### Functionality
- [ ] [Task-specific criteria]
- [ ] SUSHI compatibility verified (if applicable)
- [ ] Golden file tests pass (if applicable)

### Documentation
- [ ] Code is well-commented
- [ ] Public APIs have doc comments
- [ ] README updated (if needed)
```

---

## Next Steps

1. **Review this index** for completeness
2. **Create individual task files** for Phase 1 (Tasks 10-29)
3. **Set up CI/CD** (Task 02) to enforce quality checks
4. **Set up test harness** (Task 03) for SUSHI comparison
5. **Begin Task 10** (FHIR Package Loader)

---

**Last Updated**: 2025-11-07 (Phase 1.5 GritQL Integration added)
**Status**: Phase 1 Complete → Phase 1.5 (GritQL) & Phase 2 (Linter) Ready
