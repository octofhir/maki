# Instance Export Issues (maki vs SUSHI)

Date: 2025-11-28

## Progress

| Metric | Before | After |
|--------|--------|-------|
| Identical files | 49 | **60** |
| Different files | 301 | **290** |

## Summary

Instance exports have several critical differences that need to be addressed for SUSHI parity.

## Issues Identified

### 1. Reference Format - Missing ResourceType Prefix

**Status**: FIXED

Added expanded path-to-type mapping and fishing context lookup in `parse_reference_with_context()`.

**Fix location**: `instance_exporter.rs:1693-1753`

---

### 2. Extension URLs Not Resolved

**Status**: FIXED

- Added `fish_extension()` method to FishingContext that specifically looks for Extension StructureDefinitions
- Added slice-to-extension name mapping for common mcode/US Core extensions
- Enhanced candidate patterns with mcode- prefix

**Fix locations**:

- `fishing.rs:548-639` - New `fish_extension()` function
- `instance_exporter.rs:1184-1199` - Extended candidate patterns
- `instance_exporter.rs:1338-1354` - `map_slice_to_extension()` helper

---

### 3. Status/Intent Fields as Objects Instead of Strings

**Severity**: Critical

FHIR code elements should be simple strings, not objects:
```json
// maki
"status": "final \"final\""
"intent": {"coding": [{"code": "order", "system": "..."}]}

// SUSHI
"status": "final"
"intent": "order"
```

---

### 4. Incomplete Quantity Values

**Severity**: High

`valueQuantity` missing unit/system/code:
```json
// maki
"valueQuantity": 0.59

// SUSHI
"valueQuantity": {
  "value": 0.59,
  "code": "m2",
  "system": "http://unitsofmeasure.org",
  "unit": "square meter"
}
```

---

### 5. Array vs Single Value

**Severity**: High

Some fields incorrectly wrapped in arrays:
```json
// maki
"performedPeriod": [{"start": "2018-08-15", "end": "2018-10-25"}]

// SUSHI
"performedPeriod": {"start": "2018-08-15", "end": "2018-10-25"}
```

---

### 6. Missing Required Fields

**Severity**: High

Some fields completely missing:
- `code` - missing coding information
- `category` - missing category classification
- `identifier.system` - missing system URL

---

### 7. Extra Content in Display

**Severity**: Low

maki includes FSH comment syntax in display:
```json
// maki
"display": "// \"Radiotherapy treatment of..."

// SUSHI
(no display, or clean display text)
```

---

## Files to Investigate

- `crates/maki-core/src/export/instance_exporter.rs` - Main instance export logic
- `crates/maki-core/src/export/build.rs` - Build orchestration
- `crates/maki-core/src/semantic/fishing.rs` - Resource resolution

## Priority Order

1. Reference format (ResourceType/ prefix) - affects all references
2. Extension URL resolution - affects all extensions
3. Status/intent as strings - affects all status/intent fields
4. Quantity values - affects all measurements
5. Array vs single value - affects specific fields
6. Missing fields - profile-specific
