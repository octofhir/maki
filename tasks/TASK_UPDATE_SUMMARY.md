# MAKI Task Files Update Summary

**Date**: 2025-11-07
**Status**: In Progress
**Scope**: Comprehensive update of tasks 30+ to reflect Rowan CST architecture and add GritQL integration

---

## Completed

### ✅ New GritQL Task Files Created

1. **Task 29: Complete GritQL Pattern Execution Engine** ✅
   - File: `tasks/29_gritql_execution_engine.md`
   - 6-7 weeks effort
   - Critical foundation for custom rules
   - Removes hardcoded pattern matching
   - Implements: variable binding, predicates, field access, built-in functions

2. **Task 29.5: GritQL Code Rewriting & Autofixes** ✅
   - File: `tasks/29.5_gritql_code_rewriting.md`
   - 4 weeks effort
   - Enables autofixes from GritQL patterns
   - Implements: effect application, safety classification, validation

---

## Pending Task Files

### 🔄 New Advanced Rule Tasks (To Create)

3. **Task 29.7: GritQL Developer Experience**
   - Pattern testing framework
   - Pattern debugger CLI command
   - Performance profiler
   - IDE support for .grit files
   - 4 weeks effort

4. **Task 30.5: Documentation Rules (Enhanced)**
   - TODO/FIXME comment tracking
   - Missing inline documentation
   - Comment placement validation
   - Canonical URL consistency
   - 1 week effort

5. **Task 35.5: Project-Wide Rules**
   - Consistent naming prefix enforcement
   - Orphaned ValueSet detection
   - Missing examples for profiles
   - FHIR version compatibility
   - Package dependency validation
   - 2 weeks effort

6. **Task 36.5: Semantic Analysis Rules**
   - Circular reference detection
   - Unused ruleset detection
   - Shadowed element warnings
   - Extension context usage validation
   - 2 weeks effort

7. **Task 39.5: Advanced Lint Rules (Trivia & Patterns)**
   - Multiline string consistency
   - Comment preservation warnings
   - Token spacing validation
   - Indentation consistency
   - 2 weeks effort

---

### 🔄 Existing Task Updates Required

All tasks 30-47 need comprehensive updates with the following sections:

#### Common Updates for All Tasks:

1. **Code Examples Alignment**
   ```rust
   // OLD (incorrect):
   pub fn check_rule(entity: &dyn Entity) -> Vec<Diagnostic>

   // NEW (correct):
   pub fn check_rule(model: &SemanticModel) -> Vec<Diagnostic> {
       let Some(document) = Document::cast(model.cst.clone()) else {
           return Vec::new();
       };
       for profile in document.profiles() {
           // Use actual AST API
       }
   }
   ```

2. **FHIR Definitions Integration** (Tasks 32, 33, 36)
   ```rust
   use maki_core::canonical::CanonicalManager;

   let manager = CanonicalManager::new()?;
   manager.ensure_packages(&["hl7.fhir.r4.core@4.0.1"])?;

   let parent_cardinality = manager.get_element_cardinality(
       &parent_resource,
       &element_path
   )?;
   ```

3. **Workspace API Documentation** (Tasks 35, 36)
   ```rust
   workspace.get_profile(name: &str) -> Option<&Profile>
   workspace.has_resource_type(name: &str) -> bool
   workspace.find_instances_of(profile: &str) -> Vec<&Instance>
   ```

4. **GritQL Pattern Examples** (All Tasks)
   ```gritql
   // Show how the same rule could be written in GritQL
   Profile: $p where {
       not $p.title
   }
   ```

5. **CST Capabilities** (All Tasks)
   - Trivia-aware validation
   - CST navigation patterns
   - Incremental parsing for LSP

#### Specific Task Updates:

**Task 30: Naming Conventions**
- ✅ Partially implemented (`builtin/naming.rs` exists)
- Add: GritQL naming pattern examples
- Add: Trivia-aware rules section
- Update: API from `Entity` trait to `Document::cast()`

**Task 31: Metadata Requirements**
- ✅ Partially implemented (`builtin/metadata.rs` exists)
- Add: Canonical-manager integration
- Add: Metadata insertion using CST builder API
- Update: Field access patterns

**Task 32: Cardinality Validation**
- Add: FHIR definitions access via canonical-manager
- Add: Parent element cardinality lookup
- Document: How to query FHIR base specs

**Task 33: Binding Strength**
- Add: ValueSet binding validation with FHIR definitions
- Add: GritQL binding patterns
- Document: How to check parent binding strength

**Task 34: Required Fields**
- Add: Workspace API for profile resolution
- Add: Extension context requirements
- Add: Instance validation against profiles

**Task 35: Duplicate Detection**
- Add: Cross-file duplicate detection
- Add: GritQL duplicate patterns
- Document: Workspace-level scanning

**Task 36: Profile Consistency**
- Add: Comprehensive FHIR definitions integration
- Add: Type constraint conflict detection
- Add: Reference target validation
- Add: MustSupport propagation checks

**Task 37: Autofix Engine**
- ✅ Basic implementation exists (`autofix.rs`)
- Update: Actual `CodeSuggestion` API (not `Fix` struct)
- Add: **GritQL autofix integration** (depends on Task 29.5)
- Add: Safe/unsafe classification
- Update: CLI integration (--fix, --fix-unsafe)

**Task 38: FSH Formatter**
- ✅ Implementation exists (`cst/formatter.rs`)
- Confirm: Implementation matches task description
- Add: Performance benchmarks

**Task 39: Format Command**
- Update: CLI integration patterns
- Add: Configuration options

**Task 40: LSP Foundation**
- Add: **Incremental parsing section** (major addition)
- Document: `cst/incremental.rs` usage
- Add: Document synchronization patterns
- Add: Sub-50ms reparsing targets

**Tasks 41-47: LSP Features**
- Add: CST/semantic model integration
- Add: Real-time diagnostics with GritQL rules
- Add: Incremental parsing for performance

---

## Master Planning Documents

### 🔄 00_TASK_INDEX.md Update

Add new tasks:
- Task 29: GritQL Execution Engine
- Task 29.5: GritQL Code Rewriting
- Task 29.7: GritQL Developer Experience
- Task 30.5: Documentation Rules (Enhanced)
- Task 35.5: Project-Wide Rules
- Task 36.5: Semantic Analysis Rules
- Task 39.5: Advanced Lint Rules

Add cross-references:
- Link related tasks
- Show dependencies (e.g., Task 29 → Task 29.5 → Task 37)
- Add GritQL section to index

### 🔄 MAKI_PLAN.md Update

Major additions:
1. **New Phase 1.5: GritQL Integration** (insert before Phase 2)
   - Task 29: GritQL Execution (6-7 weeks)
   - Task 29.5: GritQL Rewriting (4 weeks)
   - Task 29.7: GritQL Developer Experience (4 weeks)
   - **Total: ~14-15 weeks**

2. **Update Phase 2: Enhanced Linter**
   - All rules can now use GritQL patterns
   - Add new advanced rule tasks (30.5, 35.5, 36.5, 39.5)
   - Update task dependencies

3. **Add CST Architecture Section**
   - Document Rowan-based CST capabilities
   - Explain green/red tree pattern
   - Show lossless parsing
   - Explain trivia preservation

4. **Update Timeline**
   - Original: 26 weeks (6 months)
   - With GritQL: ~40 weeks (9-10 months)
   - Can be parallelized in some areas

5. **Add Dependency Graph**
   - Visual representation of task relationships
   - Show which tasks can run in parallel
   - Highlight blocking dependencies

---

## Implementation Priority

### Phase 1: Critical Documentation Updates (Week 1-2)
**Goal**: Fix outdated code examples in existing tasks

1. Update tasks 30-39 code examples (2 weeks)
   - Replace `Entity` trait with `Document::cast()`
   - Update diagnostic API
   - Update autofix API

2. Add integration sections (1 week)
   - FHIR definitions integration
   - Workspace API
   - Semantic model usage

**Deliverable**: All existing task files have accurate code examples

---

### Phase 2: GritQL Foundation (Week 3-9)
**Goal**: Complete GritQL execution engine

1. ✅ Task 29 documentation complete
2. ✅ Task 29.5 documentation complete
3. Create Task 29.7 documentation (1 day)
4. Implement Task 29 (6-7 weeks actual coding)

**Deliverable**: GritQL patterns work (not hardcoded)

---

### Phase 3: Advanced Features Documentation (Week 10-12)
**Goal**: Document new advanced rule capabilities

1. Create Task 30.5 documentation (1 day)
2. Create Task 35.5 documentation (1 day)
3. Create Task 36.5 documentation (1 day)
4. Create Task 39.5 documentation (1 day)

**Deliverable**: 4 new task files for advanced rules

---

### Phase 4: LSP Updates (Week 13)
**Goal**: Update LSP tasks with incremental parsing

1. Update Task 40 with incremental parsing (2 days)
2. Update Tasks 41-47 with CST integration (3 days)

**Deliverable**: LSP tasks reflect incremental parsing capabilities

---

### Phase 5: Master Plan Updates (Week 14)
**Goal**: Update high-level planning documents

1. Update 00_TASK_INDEX.md (1 day)
2. Update MAKI_PLAN.md (2 days)
3. Review all tasks for consistency (2 days)

**Deliverable**: Complete, consistent task documentation

---

## Key Improvements from Research

### 1. GritQL Infrastructure Understanding
- 80% complete, just needs execution
- Excellent architecture (FshGritTree, FshGritNode)
- 44 unit tests passing
- **Critical gap**: Pattern execution hardcoded

### 2. CST Capabilities Identified
- Rowan-based lossless parsing
- Full trivia access (comments, whitespace)
- Incremental parsing support
- 3,504-line typed AST layer
- Green/red tree pattern

### 3. Canonical Manager Integration
- SQLite-based FHIR package storage
- ~7ms canonical URL lookups
- Batch installation API
- Dependency: `octofhir-canonical-manager`

### 4. Actual vs. Assumed APIs

| Task Assumption | Reality |
|----------------|---------|
| `Entity` trait | `Document::cast()` + specific types |
| `DiagnosticBuilder` | `Diagnostic::new()` |
| `Fix` struct | `CodeSuggestion::safe_fix()` |
| Generic FHIR access | Canonical manager integration |
| Generic workspace | Needs API documentation |

---

## Next Steps

### Immediate (This Week)
1. ✅ Create Task 29 documentation
2. ✅ Create Task 29.5 documentation
3. ⏳ Create Task 29.7 documentation
4. ⏳ Create Task 30.5, 35.5, 36.5, 39.5 documentation
5. ⏳ Update 00_TASK_INDEX.md
6. ⏳ Update MAKI_PLAN.md

### Short-term (Next Month)
1. Update all existing task files (30-47) with correct APIs
2. Add GritQL pattern examples to each task
3. Document FHIR definitions integration
4. Document workspace API

### Long-term (Next Quarter)
1. Implement Task 29 (GritQL execution)
2. Implement Task 29.5 (GritQL rewriting)
3. Implement advanced rule tasks
4. Update LSP with incremental parsing

---

## Success Metrics

### Documentation Quality
- [ ] All code examples match actual implementation
- [ ] All integration points documented
- [ ] All tasks have GritQL pattern examples
- [ ] Performance targets specified
- [ ] Testing strategies defined

### Completeness
- [ ] 7 new task files created (29, 29.5, 29.7, 30.5, 35.5, 36.5, 39.5)
- [ ] 18 existing task files updated (30-47)
- [ ] Master plan updated with GritQL phase
- [ ] Task index updated with new tasks

### Usability
- [ ] Task files are implementable (no ambiguity)
- [ ] Dependencies clearly stated
- [ ] Effort estimates reasonable
- [ ] Examples comprehensive

---

## Conclusion

This update represents a **comprehensive modernization** of MAKI's task documentation to reflect:

1. **Actual architecture**: Rowan CST, typed AST, semantic model
2. **Real APIs**: `Document::cast()`, `CodeSuggestion`, canonical manager
3. **New capabilities**: GritQL patterns, trivia awareness, incremental parsing
4. **Advanced features**: Project-wide rules, semantic analysis, documentation quality

**Total effort to complete documentation**: ~3-4 weeks
**Total effort to implement**: ~40 weeks (can be parallelized)

**ROI**: HIGH - Clear, accurate documentation enables efficient implementation and community contributions.
