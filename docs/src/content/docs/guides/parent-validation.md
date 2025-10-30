---
title: Parent Keyword Validation
description: Understanding how FSH Lint validates Profile Parent references
---

The Parent keyword validation ensures that Profile definitions reference valid base resources or profiles according to the FHIR Shorthand specification.

## Implementation

### Components

1. **FHIR Resource Registry** - Comprehensive lists of FHIR R4 and R5 base resources (~150 each)
   - Canonical URL validation
   - External IG profile detection (USCore, mCODE, etc.)

2. **Profile Validation** - Enhanced validation logic with appropriate severity levels
   - Multi-tier validation logic
   - Support for profile-to-profile inheritance

### Validation Logic

The validation follows a layered approach:

```
1. Is it a FHIR base resource? (Patient, Observation, etc.)
   → ✅ Valid

2. Is it defined locally in the symbol table or resources list?
   → ✅ Valid

3. Is it a valid canonical URL?
   → ✅ Valid (if format is correct)
   → ❌ Error (if format is invalid)

4. Does it match a known external IG pattern? (USCore*, mcode-*)
   → ⚠️ Warning (cannot verify locally)

5. Otherwise
   → ⚠️ Warning (unknown, might be typo)
```

### Supported Parent Formats

#### 1. FHIR Base Resources
```fsh
Profile: MyPatientProfile
Parent: Patient
```

#### 2. Locally-Defined Profiles
```fsh
Profile: BaseProfile
Parent: Patient

Profile: DerivedProfile
Parent: BaseProfile  // References profile defined above
```

#### 3. Canonical URLs
```fsh
Profile: CustomProfile
Parent: http://hl7.org/fhir/us/core/StructureDefinition/us-core-patient
```

#### 4. External IG Profiles
```fsh
Profile: ExtendedUSCore
Parent: USCorePatientProfile  // Warning: cannot verify locally
```

## Diagnostic Messages

### Valid Cases (No Diagnostic)
- Parent is a FHIR R4/R5 base resource
- Parent is a locally-defined profile (by name or ID)
- Parent is a valid canonical URL

### Warnings
- **Unknown Profile**: Not a FHIR resource, local profile, or recognized external IG
  ```
  Parent 'UnknownResourceType' is not a known FHIR resource, locally-defined profile, or recognized external profile
  Help: Verify the spelling, ensure the parent profile is defined, or use a canonical URL
  ```

- **External IG Profile**: Appears to be from a known external implementation guide
  ```
  Parent 'USCorePatientProfile' appears to be from an external implementation guide and cannot be verified locally
  Help: Consider using the canonical URL instead, or ensure this profile is defined in your dependencies
  ```

### Errors
- **Missing Parent**: Profile lacks required Parent keyword
  ```
  Profile 'MyProfile' must specify a Parent
  ```

- **Invalid URL**: Canonical URL format is incorrect
  ```
  Invalid canonical URL format: URL is too short to be valid
  Help: Ensure the URL follows the pattern: http(s)://domain/StructureDefinition/profile-id
  ```

## Known External IG Prefixes

The validator recognizes these common implementation guide patterns:
- `USCore*` - US Core profiles
- `mcode-*` - mCODE (Minimal Common Oncology Data Elements)
- `QICore*` - Quality Improvement Core
- `CARIN*` - CARIN Alliance profiles
- `DaVinci*` - Da Vinci Project profiles
- `PACIO*` - PACIO Project profiles
- `IPA*` - International Patient Access
- `IHE*` - Integrating the Healthcare Enterprise
- `AU*`, `UK*`, `CA*`, `CH*` - Regional profiles (Australia, UK, Canada, Switzerland)

## FHIR Version Support

The registry supports both FHIR R4 and R5:

- **R4**: ~145 base resources
- **R5**: All R4 resources plus ~30 new resources (e.g., `Transport`, `Permission`, `SubscriptionTopic`)

Version detection: Currently defaults to R4, with R5 support available via `check_profile_assignments_with_version()`.

## Testing

Run validation on test examples:

```bash
maki lint examples/test-parent-validation.fsh
```

## Future Enhancements

### Potential Improvements
1. **FHIR Version Detection**: Auto-detect FHIR version from project configuration
2. **External Profile Cache**: Download and cache external IG definitions for verification
3. **Configuration Support**: Allow users to specify known external profiles in `.makirc`
4. **Canonical URL Resolution**: Resolve and verify canonical URLs against known registries
5. **Symbol Table Enhancement**: Index profiles by both name and ID for faster lookup

### Configuration Example (Future)
```json
{
  "validation": {
    "fhirVersion": "R4",
    "knownProfiles": [
      "USCorePatientProfile",
      "USCoreCondition"
    ],
    "externalIGPaths": [
      "./node_modules/fhir-us-core"
    ]
  }
}
```

## See Also

- [Configuration Rules](/configuration/rules/) - Configure rule behavior
- [Custom Rules](/guides/custom-rules/) - Write your own validation rules
- [CLI Commands](/cli/commands/) - Command-line usage

## External References

- [FHIR Shorthand Specification - Profiles](https://hl7.org/fhir/uv/shorthand/reference.html#defining-profiles)
- [FHIR R4 Resource List](https://hl7.org/fhir/R4/resourcelist.html)
- [FHIR R5 Resource List](https://hl7.org/fhir/R5/resourcelist.html)
