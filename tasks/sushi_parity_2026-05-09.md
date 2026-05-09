# SUSHI Parity Gap Analysis — 2026-05-09

Reference SUSHI versions surveyed: **v3.10.0 → v3.19.0** (May 2024 → April 2025/26)
maki commit reviewed: **3751986d001a5e6561917c9cd4c6fddcae0dfac0** (`main`, version 0.0.4)

Note on dates: SUSHI release dates vary by source. The release-page WebFetch returned dates that put v3.19.0 at 2025-04-16 and v3.10.0 at 2024-05-02, so the surveyed window is roughly 12 months of SUSHI activity.

This audit cross-references SUSHI release notes against maki's source under `crates/maki-cli/src/`, `crates/maki-core/src/`, `crates/maki-rules/src/`, and `crates/maki-test/src/`. Items are classified **Done**, **Partial**, or **Missing**, with a priority that reflects how often a typical FSH/IG project hits the feature.

---

## Summary

- **Total gaps**: 19
- **P0 — Critical**: 4
- **P1 — High**: 6
- **P2 — Medium**: 6
- **P3 — Low**: 3

Top three findings:

1. **Cross-version extension package handling (SUSHI 3.19) is entirely absent.** maki has no concept of `hl7.fhir.uv.xver-*` packages, no `[x]`/`%5Bx%5D` URL handling, and no auto-redirect from the legacy `hl7.fhir.extensions.*` packages.
2. **`sushi init` parity is far behind.** SUSHI 3.10 added `--config <key:value>` overrides for id/canonical/status/version/releaselabel/publisher-name/publisher-url plus `-a/--auto-initialize`; maki only exposes `--default`. SUSHI 3.19 also retired `_genonce`/`_updatePublisher` scripts in favour of `_build.sh`/`_build.bat` — maki still downloads the deprecated scripts.
3. **`position` slicing discriminator (SUSHI 3.13) is missing.** `crates/maki-core/src/semantic/slicing.rs::DiscriminatorType` only enumerates `value | pattern | type | profile | exists`. Any FSH source using `^slicing.discriminator[+].type = #position` will fail validation.

---

## P0 — Critical (block compatibility for typical FSH projects)

| # | Feature | SUSHI version | maki status | Notes |
|---|---|---|---|---|
| 1 | `position` slicing discriminator type | v3.13.0 | **Missing** | `DiscriminatorType` enum in `crates/maki-core/src/semantic/slicing.rs` (lines 207–219, parse at 223–231) lacks the `Position` variant. Used by FHIR R5 IGs that rely on stable element ordering for slicing. |
| 2 | Official cross-version extension packages (`hl7.fhir.uv.xver-*`) | v3.19.0 | **Missing** | No references to `xver`, `uv.xver-r5.r4`, etc. in `crates/maki-core/src/canonical/`. SUSHI auto-redirects `hl7.fhir.extensions.r5:4.0.1` → `hl7.fhir.uv.xver-*`; maki does neither the redirect nor URL handling. |
| 3 | `sushi init -c <key:value>` config overrides + `-a/--auto-initialize` | v3.10.0 | **Missing** | `maki init` (`crates/maki-cli/src/main.rs:128–136`) only accepts `name` and `--default`. SUSHI accepts `-c id:`, `-c canonical:`, `-c status:`, `-c version:`, `-c releaselabel:`, `-c publisher-name:`, `-c publisher-url:` and `-a` for fully scripted bootstrap. |
| 4 | `sushi init` downloads `_build.sh`/`_build.bat` (not `_genonce`/`_updatePublisher`) + `fhir2.base.template` | v3.19.0 | **Missing** | `crates/maki-cli/src/commands/init.rs:399–415` still downloads `_genonce.sh/.bat` and `_updatePublisher.sh/.bat`. SUSHI 3.19 explicitly deprecated these. Switch to `_build.sh/.bat` and update IG template name. |

## P1 — High

| # | Feature | SUSHI version | maki status | Notes |
|---|---|---|---|---|
| 5 | `sushi-ignoreErrors.txt` error suppression | v3.17.0 | **Missing** | No matches for `ignoreErrors` / `ignore_errors` / `sushi-ignore` anywhere in `crates/`. SUSHI lets users add file patterns to suppress specific diagnostics, which is critical for IG authors with grandfathered violations. |
| 6 | URL-encode `[x]` → `%5Bx%5D` in cross-version extension URLs | v3.14.0 | **Missing** | Linked to gap #2. Even before adopting xver packages, SUSHI normalises `Questionnaire.versionAlgorithm[x]` to `%5Bx%5D` in extension URLs. No grep hits for `%5Bx%5D` or such encoding in maki. |
| 7 | `canonical()` resolution across all canonical-resource types | v3.14.0 | **Partial** | `crates/maki-core/src/export/profile_exporter.rs::resolve_type_to_canonical` and `differential_generator.rs` resolve StructureDefinition/ValueSet/CodeSystem only. SUSHI 3.14 expanded resolution to all canonical resources (Questionnaire, ConceptMap, NamingSystem, etc.) and added type-aware disambiguation when multiple matches exist. |
| 8 | Multiline strings in invariant `expression` and mapping comments | v3.18.0 | **Partial** | Lexer supports `"""..."""` strings (`crates/maki-core/src/cst/lexer.rs:1091–1105`), but it is unverified that the invariant and mapping rule parsers accept multiline-string tokens at the expected positions, and there are no fixtures testing it. Needs targeted parser tests for `obeys` expressions and `Mapping`-block string fields. |
| 9 | "Define a resource fully via caret rules" (root-level caret coverage) | v3.13.0 | **Partial** | `crates/maki-core/src/semantic/rules/value.rs::apply_root_caret` exists but logs `"Skipping unsupported root caret"` for many properties (line 290). SUSHI 3.13 broadened caret rules to allow defining whole resources without standard FSH constructs; verify which root caret paths maki supports vs. silently skips. |
| 10 | NPM dependency aliases (`alias@npm:packageId`) | v3.16.0 | **Done** | Implemented in `crates/maki-core/src/config/auto_dependencies.rs:110–155` and `crates/maki-core/src/config/sushi_config.rs:629–660`. Verify that the canonical manager actually fetches the aliased package version and that aliases survive into install logs. |

## P2 — Medium

| # | Feature | SUSHI version | maki status | Notes |
|---|---|---|---|---|
| 11 | Bearer-token auth via `FPL_REGISTRY_TOKEN` env var | v3.16.1 | **Missing** | No `FPL_REGISTRY_TOKEN` or `Bearer` mentions under `crates/maki-core/src/canonical/`. Required for IGs hosted on private/custom NPM registries with token auth. |
| 12 | Inline-instance assignment for primitive types | v3.13.0 | **Partial** | InstanceOf parsing exists; maki has `Instance`/`InstanceOf` keywords (`crates/maki-core/src/cst/syntax_kind.rs:45,74`). SUSHI 3.13 specifically allowed inline instance syntax for primitives — verify the parser/exporter accepts e.g. `* date = (myDateInstance)` for primitive datatypes, not just complex types. |
| 13 | ValueSet referencing a contained inline `CodeSystem` | v3.12.0 | **Unverified / likely Partial** | `valueset_exporter.rs` does emit per-system includes, but I found no test fixture covering "ValueSet that includes a CodeSystem defined inside the same ValueSet's `contained[]`". SUSHI 3.12 added explicit support; needs a fixture and exporter check. |
| 14 | Reference target type with version-pinned canonicals (`Reference(http://…\|1.2.3)`) | v3.16.4 | **Partial** | `crates/maki-core/src/semantic/mod.rs::extract_reference_targets` (lines 709–) tokenises `Reference(...)`. Verify that pipe-versioned target profiles are preserved into `targetProfile` arrays in the output StructureDefinition. |
| 15 | Wildcard patch-version detection in dependency URIs (`5.0.x`) | v3.15.1 | **Missing/Unverified** | No grep hits for wildcard patch handling (`.x`, `*` in patch position). SUSHI fixed an issue where dependency URIs with `.x` patches weren't matched as canonicals. Confirm canonical-manager handles e.g. `hl7.fhir.us.core: 5.0.x`. |
| 16 | Warn when `sushi-config.yaml` lacks `menu` and no user `menu.xml` is present | v3.11.0 | **Partial** | `menu_generator.rs` exists and `build.rs` generates `menu.xml` from config (line 717–720) and falls back if `input/includes/menu.xml` is user-provided (line 2337–2338). No explicit warning is emitted when both are absent — SUSHI logs a warning to surface accidentally-empty IG menus. |

## P3 — Low

| # | Feature | SUSHI version | maki status | Notes |
|---|---|---|---|---|
| 17 | `hl7.fhir.uv.tools` auto-dependency uses **release-specific** latest version | v3.14.0 | **Done** | `crates/maki-core/src/config/auto_dependencies.rs:96` already maps `hl7.fhir.uv.tools.{r4,r4b,r5,r6}` per FHIR release. Confirm the resolver actually fetches "latest published" rather than a hard-coded version pin. |
| 18 | `fsh-index.json` location at `fsh-generated/data/` (not `fsh-generated/`) | v3.15.0 | **Done** | `crates/maki-core/src/export/file_structure.rs:154–155` writes JSON to `data_dir().join("fsh-index.json")`; text index stays at `fsh-generated/fsh-index.txt`. Matches SUSHI 3.15 layout. |
| 19 | Logical/Resource default root-element values to prevent empty definitions | v3.11.0 | **Partial** | `crates/maki-core/src/export/logical_exporter.rs:277–290` mutates the snapshot root element. Verify that, when a Logical/Resource has no rules, the root element still gets the mandatory `min`/`max`/`type` defaults SUSHI 3.11 introduced. |

---

## SUSHI versions surveyed

| Version | Date (per release page) | Key adds relevant to maki |
|---|---|---|
| v3.19.0 | 2025-04-16 | Official cross-version extension packages, `[x]`/`%5Bx%5D` URL normalisation, `_build.sh`/`_build.bat` scripts, `fhir2.base.template`. |
| v3.18.1 | 2025-03-08 | Bug fix: IG dependency packageIds for release-specific packages. |
| v3.18.0 | 2025-02-27 | Multiline strings in invariant expressions and mapping comments; inherited extension slice fix; VS Include version retention. |
| v3.17.0 | 2024-12-27 | `sushi-ignoreErrors.txt`; extension package gets highest dep priority; uninherited extensions stop propagating to children; npm version-check timeout. |
| v3.16.5 | 2024-09-22 | Allow base types as profiles in type constraints; recognise `_build.bat`/`_build.sh` as IG publisher scripts. |
| v3.16.4 | 2024-09-11 | Export canonical versions in `Parent`/`InstanceOf`; constrain reference types with version-pinned targets. |
| v3.16.3 | 2024-07-08 | FHIR Package Loader v2.2.2 (custom NPM registry auth fixes). |
| v3.16.2 | 2024-06-30 | FPL v2.2.1 (custom NPM compat). |
| v3.16.1 | 2024-06-30 | `FPL_REGISTRY_TOKEN` env var for bearer-token auth on custom registries. |
| v3.16.0 | 2024-05-20 | NPM alias support `alias@npm:pkg` enabling multiple versions of one package. |
| v3.15.1 | 2025-05-02 (per fetch) | Fix wildcard patch (`.x`) dependency URI detection; FPL v2.1.2 downgrades unavailable `#current` to warning. |
| v3.15.0 | 2025-03-28 | `fsh-index.json` moved to `fsh-generated/data/`. |
| v3.14.0 | 2024-12-31 | `canonical()` works across **all** canonical resources in `input/*` and deps; type-aware resolution; `[x]` URL-encoding for cross-version extensions; `hl7.fhir.uv.tools` auto-dep latest release-specific. |
| v3.13.1 | 2024-12-21 | Node 18 < 18.20.0 compat fixes. |
| v3.13.0 | 2024-12-20 | `position` slicing discriminator; resources fully definable via caret rules; inline instance for primitives; FPL 2.0.0. |
| v3.12.1 | 2024-11-08 | R5 terminology/extensions auto-load for FHIR R6 ballot projects. |
| v3.12.0 | 2024-10-11 | FHIR R6 (`fhirVersion: 6.0.0-ballot2`); ValueSet referencing contained inline CodeSystems. |
| v3.11.1 | 2024-08-29 | Performance: predefined-resource processing; caret rule fix for versioned codes in ValueSets; warning on unresolvable references. |
| v3.11.0 | 2024-06-05 | `FshToFhir` API gains `snapshot` option; `sushi init` removes hard-coded `releaseLabel`; warn when `menu` + `menu.xml` both missing; Logical/Resource root-element defaults. |
| v3.10.0 | 2024-05-02 | `sushi init NAME` arg + `-c <key:value>` configs (id, canonical, status, version, releaselabel, publisher-name, publisher-url) + `-a/--auto-initialize`; `sushi build -c` overrides for version/status/releaselabel; FSHOnly defaults to IG version. |

---

## Notes on what's already in good shape

- **Caret value rules**: structurally supported (`crates/maki-core/src/semantic/rules/value.rs`, `crates/maki-rules/src/builtin/caret_path.rs` lint, syntax kinds `CaretValueRule = 257`, `CodeCaretValueRule = 304`).
- **Mapping/Invariant/RuleSet/Logical/Resource/Extension/CodeSystem/ValueSet**: all are first-class CST/AST nodes (`crates/maki-core/src/cst/syntax_kind.rs`) with corresponding exporters under `crates/maki-core/src/export/`.
- **`build -c version:... -c status:... -c releaselabel:...`**: implemented in `maki-cli/src/main.rs:117–124` (matches SUSHI 3.10).
- **Multiline (`"""…"""`) string lexing**: present in `crates/maki-core/src/cst/lexer.rs:1091–1105`; gap #8 is about parser-position acceptance, not lexing.
- **NPM aliases**: `alias@npm:pkg` recognised (gap #10 promoted to verified-Done).
- **`menu.xml` generation from `sushi-config.yaml::menu`**: present in `crates/maki-core/src/export/menu_generator.rs` and wired in `build.rs:717–720`.
- **`fsh-generated/data/fsh-index.json` layout**: matches SUSHI 3.15 (gap #18, verified-Done).
- **Predefined resource loading**: `build.rs:732–733` + `predefined_resources.rs` cover SUSHI's `input/resources` / `input/examples` style ingestion (verify behaviour matches SUSHI's conflict-detection rules).
- **VS Include version retention** (SUSHI 3.18): already preserved in `crates/maki-core/src/export/valueset_exporter.rs:553–582`.

## Out-of-scope / informational

- **`FshToFhir` API `snapshot` option (SUSHI 3.11)** — maki exposes its own programmatic API via `maki-core::lib`; SUSHI's JS-API shape isn't a parity target. The CLI flag `maki build --snapshot` already exists (`main.rs:81–83`).
- **Security updates to axios/lodash (SUSHI 3.19)** — N/A, maki is Rust.
- **FPL v2.x integration (SUSHI 3.13–3.16)** — maki uses `octofhir-canonical-manager` instead of FPL; only the user-visible features (NPM aliases, registry tokens, wildcard patches) need parity.
- **Logging fixes for `Parent` lacking mapping (SUSHI 3.10)** — diagnostic-only, low value to backlog separately.
- **Snapshot mapping fix when parent lacks mappings (SUSHI 3.19)** — verify with a regression fixture; not a backlog item unless reproducible in maki.
- **maki-only extensions** that SUSHI doesn't have (and so are not parity items): GritQL-based rules (`crates/maki-rules/src/gritql/`), built-in lints (`crates/maki-rules/src/builtin/`), unified `.makirc` config, autofix engine, SARIF/GitHub-Actions diagnostic output, gofsh sub-command, LSP scaffolding (`crates/maki-lsp/`).

---

## Suggested next actions (not part of the gap list, but the obvious follow-ups)

1. Add `Position` to `DiscriminatorType` and a parser test that round-trips `^slicing.discriminator[+].type = #position`.
2. Replace the publisher-script URLs in `init.rs` with the `_build.sh`/`_build.bat` equivalents and bump the IG template name.
3. Extend `maki init` clap definition to accept repeatable `-c key:value` flags and an `-a/--auto-initialize` shortcut, mirroring `build`'s `parse_config_override`.
4. Wire a `sushi-ignoreErrors.txt` loader into the diagnostics pipeline (read once at lint/build start, filter by code/path).
5. Spike on `hl7.fhir.uv.xver-*` package layout and add a redirection table in `auto_dependencies.rs` for the legacy `hl7.fhir.extensions.*` ids.
