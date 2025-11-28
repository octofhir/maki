# Plan: Instance Export SUSHI Parity - Comprehensive Fix Plan

## Current Status

| Metric | Before Session | After Session |
|--------|----------------|---------------|
| Identical files | 49 | **90** |
| Different files | 301 | **260** |

### Resource Type Breakdown

| Type | Identical | Different | Parity % | Priority |
|------|-----------|-----------|----------|----------|
| MedicationRequest | 11 | 0 | **100%** | Done |
| BodyStructure | 4 | 0 | **100%** | Done |
| Group | 1 | 0 | **100%** | Done |
| Observation | 15 | 84 | 15% | HIGH |
| ValueSet | 42 | 61 | 41% | HIGH |
| StructureDefinition | ~0 | 53 | 0% | MEDIUM |
| Condition | 9 | 4 | 69% | LOW |
| Bundle | ~0 | 2 | 0% | HIGH |

---

## HIGH PRIORITY Issues

### Issue 1: Profile Constraint Resolution for Observations (84 files)

**Status**: Implemented in `instance_exporter.rs` (profile chain fixed/pattern pre-seeded before rules). Needs parity diff rerun.

**Problem**: Observation instances are missing `code` and `category` fields that should be inherited from parent profiles.

**Root Cause Analysis**:
- When FSH defines `InstanceOf: CancerDiseaseStatus`, the instance should inherit fixed values from the profile
- The profile defines `code = LNC#88040-1` but instances don't get this value
- SUSHI resolves profile constraints and applies them to instances; maki doesn't

**Example**:
```fsh
// Profile defines:
Profile: CancerDiseaseStatus
Parent: Observation
* code = LNC#88040-1 "Disease status"

// Instance should inherit code:
Instance: cancer-disease-status-jenny-m
InstanceOf: CancerDiseaseStatus
// maki output: missing "code" field
// SUSHI output: has "code": {"coding": [...], "text": "Disease status"}
```

**Fix Locations**:
- `crates/maki-core/src/export/instance_exporter.rs`
- Add profile constraint resolution in `export_instance()` or `apply_rules()`

**Implementation**:
1. When exporting an instance, look up its parent profile
2. Extract fixed values from profile's differential elements
3. Apply these values to the instance JSON before rule processing
4. Handle multi-level inheritance (Instance -> Profile -> Parent Profile -> Base)

**Impact**: 84 Observation files + potentially other instance types

---

### Issue 2: ValueSet Export Issues (61 files)

**Status**: Implemented. Added NCIT alias, normalized `descendant-of`, and handled bare `codes from system` includes.

#### 2a. Missing NCIT Code System Alias

**Problem**: `NCIT` (NCI Thesaurus) alias not recognized, resulting in unresolved system URLs.

**Fix Location**: `crates/maki-core/src/export/valueset_exporter.rs:74-96`

```rust
// Add to CODE_SYSTEM_ALIASES:
"NCIT" => "http://ncicb.nci.nih.gov/xml/owl/EVS/Thesaurus.owl",
```

#### 2b. Filter Operator Spelling

**Problem**: Using `"descendent-of"` instead of `"descendant-of"` in filter rules.

**Fix Location**: `crates/maki-core/src/export/valueset_exporter.rs:1281-1283`

```rust
// Change:
"descendsFrom" | "descendent-of" => "descendant-of",
// The FHIR spec uses "descendant-of" (with 'a')
```

#### 2c. Missing "include codes from system" Handler

**Problem**: FSH syntax `* include codes from system X` not properly handled.

**FSH Example**:
```fsh
* include codes from system http://ncicb.nci.nih.gov/xml/owl/EVS/Thesaurus.owl
```

**Expected FHIR Output**:
```json
{"system": "http://ncicb.nci.nih.gov/xml/owl/EVS/Thesaurus.owl"}
```

**Fix**: Add handler in `parse_value_set_rules()` for bare system inclusion.

---

### Issue 3: Bundle Resource Embedding (2 files)

**Status**: Implemented. Bundle entries now inline registered instances instead of leaving string references.

**Problem**: Bundle entries contain string references instead of full embedded resources.

**Maki Output**:
```json
{"fullUrl": "...", "resource": "urn:uuid:..."}  // String reference
```

**SUSHI Output**:
```json
{"fullUrl": "...", "resource": {"resourceType": "Patient", ...}}  // Full object
```

**Root Cause**: Bundle exporter treats inline instance references as strings rather than resolving them to full resource objects.

**Fix Location**: `crates/maki-core/src/export/instance_exporter.rs` (Bundle handling)

**Implementation**:
1. When processing Bundle.entry[n].resource, check if value is a reference
2. Resolve the reference to the actual Instance definition
3. Export that instance inline as a full JSON object

---

## MEDIUM PRIORITY Issues

### Issue 4: StructureDefinition Differences (53 files)

**Status**: Version cleared unless explicitly set; top-level extensions sorted; inherited mappings removed. Differential-only already default. Field ordering still pending.

#### 4a. Version Field Inclusion

**Problem**: Maki includes `version` field; SUSHI omits it when not explicitly set.

**Decision**: Match SUSHI behavior - only include version when explicitly defined in FSH.

**Fix Location**: `crates/maki-core/src/export/profile_exporter.rs`

#### 4b. Extension Ordering

**Problem**: Extensions appear in different order.

**Fix**: Sort extensions by URL for deterministic output.

#### 4c. Snapshot vs Differential

**Problem**: Maki generates snapshot elements; SUSHI generates differential only.

**Decision**: Generate differential only by default (SUSHI parity).

**Fix Location**: `crates/maki-core/src/export/profile_exporter.rs`

#### 4d. Mapping Section

**Problem**: Maki includes inherited mappings; SUSHI includes only FSH-defined mappings.

**Decision**: Match SUSHI behavior for parity.

---

## LOW PRIORITY Issues

### Issue 5: Condition Field Ordering (4 files)

**Problem**: Fields appear in different order (e.g., `stage.assessment` position).

**Impact**: Cosmetic only - valid FHIR either way.

**Fix**: Implement deterministic field ordering matching SUSHI's output.

---

## Implementation Order

| Phase | Issue | Files Affected | Estimated Impact |
|-------|-------|----------------|------------------|
| 1 | Profile Constraint Resolution | instance_exporter.rs | +84 files | ✅ implemented (rerun diff) |
| 2 | ValueSet NCIT + Operators | valueset_exporter.rs | +40 files | ✅ implemented |
| 3 | Bundle Resource Embedding | instance_exporter.rs | +2 files | ✅ implemented |
| 4 | StructureDefinition Cleanup | profile_exporter.rs | +53 files | ✅ metadata/mapping/extension updates (rerun diff) |
| 5 | Field Ordering | various | +4 files | ⏳ pending |

**Expected Final State**: ~350/350 identical (100% parity)

---

## Critical Files to Modify

| File | Changes |
|------|---------|
| `crates/maki-core/src/export/instance_exporter.rs` | Profile constraint resolution, Bundle embedding |
| `crates/maki-core/src/export/valueset_exporter.rs` | NCIT alias, filter operators, codes from system |
| `crates/maki-core/src/export/profile_exporter.rs` | Version field, extension ordering, differential-only |

---

## Testing Strategy

1. **Build maki**:
   ```bash
   cargo build
   ```

2. **Run on mcode-ig**:
   ```bash
   ./target/debug/maki build mcode-ig/
   ```

3. **Compare with SUSHI output**:
   ```bash
   diff -r mcode-ig/fsh-generated/resources mcode-ig/fsh-generated-sushi/resources | grep "^diff" | wc -l
   ```

4. **Run test suite**:
   ```bash
   cargo test --workspace
   ```

_Note_: `cargo test -p maki-core valueset_exporter_unit_test` currently fails on existing `FishingContext::extract_metadata` test harness gaps (not from parity work). Address or mark ignored before final verification.

---

## Design Decisions

1. **Profile Constraint Inheritance**: Full chain resolution matching SUSHI behavior
   (Instance -> Profile -> Parent Profile -> Base)

2. **Snapshot Generation**: Differential-only for SUSHI parity (can add snapshot flag later)

3. **Field Ordering**: Match SUSHI for deterministic comparison
