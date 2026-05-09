# FSH Spec Coverage Audit — 2026-05-09

Spec version surveyed: **FSH 3.0.0** (https://hl7.org/fhir/uv/shorthand/, Language Reference)
maki commit reviewed: **3751986d001a5e6561917c9cd4c6fddcae0dfac0** (`main`, version 0.0.4)

This audit cross-references every named construct in the FSH Language Reference against maki's parser
(`crates/maki-core/src/cst/`), semantic layer (`crates/maki-core/src/semantic/`), exporters
(`crates/maki-core/src/export/`), and lint-rule registry (`crates/maki-rules/src/builtin*`).
Severity reflects how often a real-world FSH project hits the gap, not implementation cost.

A companion document (`tasks/sushi_parity_2026-05-09.md`) tracks divergence from the SUSHI implementation;
this audit is scoped strictly to the **published FSH language specification**.

---

## Summary

- **Critical gaps**: 4
- **High gaps**: 7
- **Medium gaps**: 6
- **Low gaps**: 4

Top three findings:

1. **Slicing discriminators are not emitted into FHIR output.** `crates/maki-core/src/export/profile_exporter.rs:1745-1751` explicitly leaves `base_elem.slicing` unset (`TODO: Add slicing discriminator for extensions`), so any profile that uses non-extension slicing or that needs an explicit `^slicing.discriminator[+].type = #value/#pattern/#type/#profile/#exists` will export a slice list with no `slicing.discriminator`, breaking IG validation.
2. **Indented rule path inheritance is not implemented.** The spec (FSH §FSH Rules) requires that an indented rule with no leading `*`-path inherit the path from the prior unindented rule. `crates/maki-core/src/cst/parser.rs::parse_rule` and the AST layer treat each `*`-rule independently — no path-context stack is built. Real-world IGs that lean on indented rule blocks (e.g. `* code` followed by `  * coding[+].system = ...`) will silently mis-parse paths.
3. **Mappings, contentReference elements, and several rule types do not survive into exported StructureDefinitions.** `MappingExporter::apply_mapping` (`crates/maki-core/src/export/mapping_exporter.rs:113-260`) is wired, but the `* path -> "target"` rule type inside a profile/extension is dropped (`profile_exporter.rs:1307-1309`). `AddCRElementRule` is parsed (`parser.rs:2664-2719`) but `LogicalExporter` only handles `AddElementRule` (`logical_exporter.rs:347, 565`). `Resource` parents other than `Resource`/`DomainResource` are not validated.

---

## Coverage matrix

| Section / Construct | Parses | Semantic | Exports | Lints | Severity | Notes |
|---|---|---|---|---|---|---|
| Aliases (`Alias: $x = url`) | Yes | Yes | n/a | Partial | Low | Lex/parse at `cst/parser.rs:530-563`; resolved in `semantic/alias.rs`. Lints: `malformed-alias`, `duplicate-alias`. No URL-shape lint. |
| Aliases without `$` (Profile/Extension aliases) | Yes | Yes | n/a | No | Low | `semantic/alias.rs:8-19` accepts both forms. |
| Profile (`Profile`, `Parent`, `Id`, `Title`, `Description`) | Yes | Yes | Yes | Partial | n/a | `cst/parser.rs:181-244`; full export via `profile_exporter.rs`. |
| Extension (simple, `value[x] only`) | Yes | Yes | Yes | Partial | n/a | `extension_exporter.rs:135`. |
| Extension (complex / sliced sub-extensions) | Yes | Yes | Partial | No | High | Sub-extension slices created (`profile_exporter.rs:1748-1789`) but discriminator object on parent element is **not** written. |
| Extension `Context:` keyword | Yes | Yes | Partial | Partial | Medium | Direct `Context: Patient` parsed (`parse_context_clause`, `parser.rs:1109`); `^context[+].type/.expression` caret form handled (`extension_exporter.rs:265-520`). FHIRPath `Context: "(A | B).code"` not validated. |
| ValueSet (Id/Title/Description) | Yes | Yes | Yes | Partial | n/a | `valueset_exporter.rs:411`. |
| ValueSet `* include #code` / direct concept | Yes | Yes | Yes | No | n/a | `parse_vs_concept_component`, `parser.rs:3104`. |
| ValueSet `* include codes from system X` | Yes | Yes | Yes | No | n/a | `parse_vs_from_system`, `parser.rs:3216`. |
| ValueSet `* include codes from valueset Y` | Yes | Yes | Yes | No | n/a | `parse_vs_from_valueset`, `parser.rs:3238`. |
| ValueSet `where … is-a/descendent-of/=/in/generalizes/regex` | Yes | Partial | Yes | No | Medium | Operator stored as opaque ident (`parser.rs:3313-3327`); maki does not validate operator vocabulary. |
| ValueSet `system\|version#code` versioned coding | Partial | Partial | Partial | No | Medium | Lexer keeps `\|version` glued to identifier; no dedicated CST node. |
| ValueSet exclude rules | Yes | Yes | Yes | No | n/a | Same path as include; `parser.rs:3086`. |
| CodeSystem (`* #code "display" "definition"`) | Yes | Yes | Yes | No | n/a | `parser.rs:351-414`; `codesystem_exporter.rs:183`. |
| CodeSystem hierarchical concepts via `* #parent #child` | Partial | Partial | Partial | No | Medium | `parse_concept_as_path_rule` (`parser.rs:2989`) records sequence, but hierarchy rebuild during export not verified by tests. |
| CodeSystem hierarchical concepts via indentation | No | No | No | No | High | Indented hierarchy syntax requires path-context inheritance (see Critical #2). |
| CodeSystem `* #code ^designation/^property` (code caret rules) | Yes | Partial | Partial | No | Medium | `parse_code_caret_rule`, `parser.rs:2879`; export of `^property` arrays into CodeSystem.property/concept.property is not unit-tested. |
| CodeSystem `* #code insert RuleSetName` | Yes | Partial | Unknown | No | Medium | `parse_code_insert_rule`, `parser.rs:2886`. RuleSet expansion through CodeInsertRule not exercised in `crates/maki-core/tests/`. |
| Instance (`InstanceOf:`, rules) | Yes | Yes | Yes | Partial | n/a | `instance_exporter.rs:395`. |
| Instance `Usage: #example` / `#definition` / `#inline` | Yes | Yes | Partial | No | High | Parsed via `parse_usage_clause` (`parser.rs:1091`); inline contained-instance materialization (Bundle.entry, contained[]) is partial — see High #3. |
| Invariant (`Severity`, `Description`, `Expression`, `XPath`) | Yes | Yes | Partial | No | Medium | `semantic/invariant.rs`. Invariants attach to elements via `obeys` (`profile_exporter.rs:1905`), but XPath/Expression rule round-tripping into ElementDefinition.constraint is partial. |
| Mapping (`Mapping:`, `Source`, `Target`) | Yes | Yes | Yes | No | n/a | `mapping_exporter.rs:113-260`. |
| Mapping rules `* path -> "target" "comment" #lang` | Yes | Yes | Partial | No | Critical | Inside Mapping items, exported via MappingExporter; **inside profiles**, the same `* path -> "target"` form is silently dropped (`profile_exporter.rs:1307-1309`). |
| RuleSet (unparameterized) | Yes | Yes | n/a | No | n/a | `parser.rs:565-656`; `semantic/ruleset.rs`. |
| RuleSet (parameterized + `{param}` substitution) | Yes | Yes | n/a | No | n/a | `parse_parameter_list` `parser.rs:657`; bracketed-param escapes handled (`lexer.rs:78-109`). |
| `insert RuleSet(args)` rule | Yes | Yes | n/a | No | n/a | `parse_insert_rule_body`, `parser.rs:783`. |
| Logical model (`Logical:`, `Parent: Element/Resource/DomainResource/Base`) | Yes | Yes | Yes | No | Medium | `logical_exporter.rs:69-133`. Default parent is `Element` (line 83) — spec §Defining Logical Models says default is **`Base`**. |
| Logical model `Characteristics: #can-bind, #has-units…` | Yes | Yes | Yes | No | n/a | `logical_exporter.rs:300` applies them as extensions. |
| AddElement rule (`* path 0..1 string "short" "def"`) | Yes | Yes | Yes | No | n/a | `parser.rs:2417-2469`; `logical_exporter.rs:565`. |
| AddCRElement rule (`* path 0..1 contentReference URL "short"`) | Yes | Partial | No | No | High | Parsed (`parser.rs:2664`); `LogicalExporter::apply_rule` matches only `AddElement` (`logical_exporter.rs:347`) — `AddCRElement` falls through to the warn-and-skip arm. |
| Resource (`Resource:`, custom resource) | Yes | Yes | Yes | No | Medium | `logical_exporter.rs:136`. Default parent `DomainResource`; spec restricts parents to `DomainResource\|Resource`, but this restriction is not enforced. |
| Caret rules `* ^url = …`, `* ^status = …`, `* element ^short = …` | Yes | Yes | Yes | Partial | n/a | `parser.rs:1198-1217` (root caret), `apply_caret_value_rule` `profile_exporter.rs:1959`. Lint: `caret_path::INVALID_CARET_PATH`. |
| Caret rule on root `. ^abstract` | Yes | Yes | Partial | No | Medium | `parse_path` `parser.rs:1873-1877` accepts `.` as root path; export coverage not asserted in tests. |
| Slicing — `contains` rule with `and` items | Yes | Yes | Yes | Partial | n/a | `parser.rs:1312-1396`; lint `slice_name_collision_rule`. |
| Slicing — `^slicing.discriminator[+].type/.path` caret rules | Yes | Yes | **No** | No | Critical | Parsed as caret rules but `profile_exporter.rs:1745` explicitly TODOs this. Discriminator metadata never reaches the StructureDefinition. |
| Slicing — `^slicing.rules` (`#open\|#closed\|#openAtEnd`) | Yes | Yes | No | No | Critical | Same path as discriminator; not propagated. |
| Re-slicing `slice[reslice]` | Partial | Partial | No | No | Medium | Bracket parser tolerates chained `[a][b]` but `path_resolver.rs::Bracket` does not represent reslice semantics distinctly. |
| Mixins via `insert RuleSet` (multiple) | Yes | Yes | Partial | No | Medium | Multiple insert rules expand fine, but circular detection (`parser.rs::ruleset_dependencies`) only catches direct loops, not multi-step chains. |
| Indented rules (path context inheritance) | **No** | No | No | No | Critical | No code path inherits the prior rule's path. Only `contains` rule body skips newlines (`parser.rs:1318`). |
| Top-level path | Yes | Yes | Yes | Partial | n/a | `path_resolver.rs:1064`. |
| Nested element path `a.b.c` | Yes | Yes | Yes | No | n/a | `parse_path` `parser.rs:1881`. |
| Reference path `performer[Practitioner]` | Yes | Partial | Partial | No | Medium | Bracket consumed (`parser.rs:1930`) but the reference-disambiguation form is not modeled distinctly from a slice; resolution may collide with slice names. |
| Choice `[x]` paths (`valueString`, `onsetDateTime`) | Yes | Yes | Yes | No | n/a | Bracket parser handles `[x]`; type-suffix expansion uses `path_resolver`. |
| Numerical array index `name[0]` | Yes | Yes | Yes | No | n/a | `path_resolver::Bracket::Index`. |
| Soft indexing `[+]`, `[=]` | Yes | Yes | Yes | No | n/a | `path_resolver.rs:96-114, 819-820`; `instance_exporter.rs:72`. |
| Sliced array path `component[respirationScore]` | Yes | Yes | Yes | No | n/a | Bracket-as-slice-name resolution in `path_resolver`. |
| Resliced array path `component[parent][child]` | Partial | Partial | Partial | No | Medium | See "Re-slicing" row. |
| Extension paths `extension[name]`, `extension[url]`, nested | Yes | Yes | Yes | No | n/a | `path_resolver` walks bracket sequences. |
| Coded value `#code`, `#"with spaces"` | Yes | Yes | Yes | No | n/a | `lex_code_literal`, `lexer.rs:756`. |
| Coded value `system#code` | Yes | Yes | Yes | No | n/a | `parse_vs_code` and value-expression parser. |
| Coded value `system\|version#code "display"` | Partial | Partial | Partial | No | High | `\|version` not split into a structured node — fragile against spaces and version literals; see High #4. |
| UCUM Quantity `5.4 'mg' "display"` | Yes | Yes | Yes | No | n/a | `parse_quantity_value`, `parser.rs:1794`. |
| Quantity from arbitrary CodeSystem `5.4 SCT#kg "kg"` | Partial | Partial | Partial | No | Medium | The numeric+code form is not explicitly modeled; falls through generic value-expression. |
| Ratio (`5 'mg' : 10 'mL'`) | Yes | Partial | Partial | No | Medium | `parse_ratio_value`, `parser.rs:1822`; export to FHIR `Ratio` data type not asserted in tests. |
| Reference `Reference(Patient or Practitioner)` | Yes | Yes | Yes | No | n/a | `parse_reference_with_or`, `parser.rs:1605`. |
| CodeableReference | Yes | Yes | Partial | No | Medium | `parse_codeable_reference`, `parser.rs:1632`. Profile/extension export of CodeableReference targets not unit-tested. |
| Canonical `Canonical(item)` / `Canonical(item\|1.0.0)` | Yes | Partial | Partial | No | Medium | `parse_canonical_with_version`, `parser.rs:1557`; the `\|version` separator detection is heuristic (line 1593) and may break against spaces. |
| Triple-quoted strings `""" … """` | Yes | Yes | Yes | No | n/a | `lex_string`, `lexer.rs:1091-1108`. Spec-mandated indentation-aware dedent rule (FSH §Language Basics) is **not** applied — body is taken verbatim. |
| Escape sequences `\\n \\r \\t \\"` | Partial | n/a | n/a | No | Medium | `lex_string` only skips `\\X` pairs (lines 1115-1119); `validate_string_escape_sequences` (`parser.rs:1455`) exists but its rule set vs the FSH spec was not verified. |
| Flag rules `MS SU TU N D ?!` | Yes | Yes | Yes | No | n/a | `syntax_kind.rs:130-143`; `apply_flag_to_element` `profile_exporter.rs:1418`. |
| Cardinality `0..1`, `1..*`, `1..1` | Yes | Yes | Yes | Yes | n/a | `parse_cardinality`, `parser.rs:1958`; lint `invalid_cardinality_rule`. |
| Binding strengths `(required\|extensible\|preferred\|example)` | Yes | Yes | Yes | Partial | n/a | `syntax_kind.rs:120-127`; lints `binding-strength-present`, `binding-strength-weakening`. |
| Only rule with multiple types `only A or B or Reference(C)` | Yes | Yes | Yes | No | n/a | `parse_lr_rule`/`parse_sd_rule_body` consume `or` chains. |
| Obeys rule `* obeys inv1 and inv2` (multi-invariant) | Yes | Yes | Yes | No | n/a | `parse_sd_rule_body` consumes `and` chains; `apply_obeys_rule` `profile_exporter.rs:1905`. |
| Obeys rule on root (`* obeys inv-1`) | Yes | Yes | Yes | No | n/a | `parser.rs:1244-1250`. |
| Comments `//`, `/* */` | Yes | n/a | n/a | No | n/a | `lexer.rs:166-220`; preserved as trivia. |
| `sushi-config.yaml` core fields | n/a | Yes | Yes | No | n/a | `config/sushi_config.rs:37-204` covers id/canonical/name/title/version/fhirVersion/status/dependencies/parameters/pages/menu/license/copyrightYear/publisher/contact/jurisdiction/resources/groups/templates/FSHOnly/applyExtensionMetadataToRoot/instanceOptions. |
| `ig.ini`, `input/fsh/`, `predefined-resources` | Partial | n/a | Partial | No | Medium | `export/file_structure.rs`, `predefined_resources.rs`. ig.ini parsing not located in source tree. |

---

## Critical gaps

### 1. Slicing discriminator metadata is never written to exported StructureDefinitions

- **Spec**: §Slicing requires a slice's parent element to declare `slicing.discriminator[].type`, `.path`, and `slicing.rules` (`#open`, `#closed`, `#openAtEnd`). Authors set these via `^slicing.*` caret rules.
- **maki**: `crates/maki-core/src/export/profile_exporter.rs:1745-1751` contains a literal `// TODO: Add slicing discriminator for extensions` and a comment "`base_elem.slicing would be set here in full implementation`". `differential_generator.rs:1468-1471` mentions extension auto-URL discriminators only.
- **Effect**: Any FSH file that uses non-extension slicing (e.g. `Observation.component`, `Bundle.entry` profiling) — extremely common — will export a slice list without the FHIR-required `slicing` object on the parent. Validators (HL7 IG Publisher, fhir-validator) will reject the StructureDefinition.

### 2. Indented rules do not inherit path context

- **Spec**: §FSH Rules — *"indented rules MUST be indented using standard space characters"* and the indented form lets authors omit the leading path segments shared with the previous rule.
- **maki**: `parse_rule` (`crates/maki-core/src/cst/parser.rs:1196-1431`) treats every `*`-rule as a fully-qualified standalone rule. There is no parser/AST stack tracking the previous rule's resolved path. Only inside a `contains` rule body does `parser.rs:1318` skip newlines, and that is solely to allow multi-line `and …` lists, not path inheritance.
- **Effect**: Common idiom

  ```fsh
  * code.coding ^slicing.discriminator[+].type = #pattern
    * ^slicing.discriminator[=].path = "system"
    * ^slicing.rules = #open
  ```

  is treated as three root-scoped caret rules instead of three rules sharing the `code.coding` parent. The second/third rules will be applied to the wrong path or silently no-op.

### 3. `Mapping` rules inside Profiles/Extensions are dropped

- **Spec**: §FSH Rules / §Defining Mappings — element-level mapping rules (`* code -> "Patient.code"`) are valid both inside a top-level `Mapping:` item and as standalone rules inside Profile/Extension/Logical/Resource items (the latter contributes to `StructureDefinition.mapping[].element[]`).
- **maki**: `profile_exporter.rs:1307-1309` matches `Rule::Mapping(_)` and comments *"Mapping rules are handled by MappingExporter, not ProfileExporter"* — but `MappingExporter::apply_mapping` (`mapping_exporter.rs:196`) only iterates rules belonging to a `Mapping` AST node, never rules nested inside a Profile/Extension. There is no second pass that collects in-profile mapping rules.
- **Effect**: Profiles authored with inline `* path -> "target"` rules export StructureDefinitions whose `element[].mapping` is empty. SUSHI handles this correctly.

### 4. Resource parent constraint and Logical default parent diverge from spec

- **Spec**: §Defining Resources — only `Resource` and `DomainResource` are valid parents. §Defining Logical Models — default parent is **`Base`** if `Parent` omitted.
- **maki**: `logical_exporter.rs:83` uses `unwrap_or_else(|| "Element".to_string())` for Logical's default; line 150 defaults Resource to `DomainResource` (correct) but never validates that an explicit Resource parent is one of `Resource`/`DomainResource`. The spec also requires `Logical` parents to be `Element`, `Resource`, `DomainResource`, `Base`, or another logical model — no validator exists for this either.
- **Effect**: A Logical model without `Parent:` produces a StructureDefinition with `baseDefinition = Element` instead of `Base`, which IG Publisher will reject. A Resource with `Parent: Patient` will silently produce a malformed StructureDefinition.

---

## High gaps

### 1. Non-extension slicing has no discriminator emission path

Same root cause as Critical #1, but called out separately because the failure mode differs: for **extension** slicing the discriminator is implicit (`url`) and SUSHI auto-generates it; for value/pattern/type/profile/exists slicing the author must specify it via caret rules, and maki's caret-rule application has no special branch for `slicing.*` paths.

### 2. CodeSystem hierarchical concepts via indentation

- **Spec**: §Defining Code Systems — preferred indented form

  ```fsh
  * #parent "Parent"
    * #child "Child"
  ```

- **maki**: Concepts are parsed as flat sequences (`parser.rs:2953-2983`). The two-code form `* #parent #child` is recognized (`parse_code_sequence`, `parser.rs:2926`), but indented `#child` becomes a sibling concept, not a child. CodeSystem export builds `concept[]` linearly without a parent-stack.
- **Effect**: Any multi-level CodeSystem authored with indentation will export with a flat concept list and a missing `concept[].concept[]` nesting.

### 3. `Usage: #inline` instances and `contained[]` materialization

- **Spec**: §Defining Instances — `#inline` instances are inserted into another resource's `contained[]` array or `Bundle.entry[]` (the spec mandates relative-reference rewriting).
- **maki**: `instance_exporter.rs` handles `#example` and `#definition` paths but I could not find a code path that auto-promotes inline instances into a host instance's `contained[]` or rewrites references. `register_instance`/`get_instance` (lines 209, 229) suggest the registration mechanism exists, but the materialization step is partial.
- **Effect**: Inline-instance composition (typical for CapabilityStatement, Bundle examples) does not produce the expected nested JSON.

### 4. Versioned coding `system|version#code` is not a structured CST node

- **Spec**: Codings carry an optional version after a pipe. Used pervasively in US Core (e.g. `http://snomed.info/sct|2023-09-01#…`).
- **maki**: `lexer.rs` does not split `|version` from the system token; the parser's `parse_canonical_with_version` (`parser.rs:1557-1601`) acknowledges this is a "lexer limitation" (lines 1594-1595) and tries to glue the version on best-effort.
- **Effect**: Version metadata may be lost or corrupted on export; downstream consumers comparing system+version to determine binding scope will see an unversioned coding.

### 5. `AddCRElement` rule (contentReference) drops in Logical/Resource export

- **Spec**: §Defining Logical Models — `* path 0..1 contentReference URL "short" "definition"` adds an element whose type is a `contentReference` to another element, not a datatype.
- **maki**: Parser support exists (`parser.rs:2664-2719`), AST node `AddCRElementRule` is present (`syntax_kind.rs:321`), but `LogicalExporter::apply_rule` (`logical_exporter.rs:347`) only matches `Rule::AddElement(_)` — there is no `Rule::AddCRElement(_)` arm. A search for `AddCRElement` in `logical_exporter.rs` returns no hits.
- **Effect**: `contentReference` elements are silently dropped from exported Logical/Resource StructureDefinitions.

### 6. Triple-quoted-string indentation normalization

- **Spec**: §Language Basics — triple-quoted strings strip a leading whitespace-only line, a trailing whitespace-only line, and de-indent the remainder by the minimum common indent.
- **maki**: `lex_string` (`lexer.rs:1091-1109`) returns the raw substring including leading/trailing whitespace and the original indentation; no normalization pass runs before export.
- **Effect**: Markdown content (e.g. `^purpose = """ … """`) keeps stray indentation when serialized into the FHIR JSON.

### 7. ValueSet `where` operator vocabulary not validated

- **Spec**: §Defining Value Sets — operators are restricted to `=`, `is-a`, `descendent-of`, `is-not-a`, `regex`, `in`, `not-in`, `generalizes`, `child-of`, `descendent-leaf`, `exists` (FHIR R4 list).
- **maki**: `parse_vs_filter_operator` (`parser.rs:3313`) accepts any identifier or `=`. There is no semantic check that the operator is in the allowed set, and no lint rule.
- **Effect**: Typos like `descended-of` parse cleanly and produce a malformed ValueSet.compose.include.filter[].op.

---

## Medium gaps

### 1. Reference-path disambiguation collides with slice paths

`performer[Practitioner]` (a reference disambiguator) and `slice[mySlice]` (a slice access) parse identically. `path_resolver::Bracket` enums treat both as a generic bracket. No code path distinguishes them when resolving against a reference vs an array.

### 2. Caret rules on root `.` element

`parse_path` accepts `.` (`parser.rs:1873-1877`) and the AST preserves it, but the export side (`apply_caret_value_rule`) does not specifically route `.` to "root element of the structure definition" — it relies on path_resolver, which was not seen to handle the lone-dot case.

### 3. `Quantity` from non-UCUM systems

The form `155 SCT#kg "kg"` (Quantity with a code-system-coded unit, FSH §Language Basics) is parsed via the generic value-expression but not assigned a dedicated AST node distinct from a bare number-and-coding pair. Export to `Quantity { value, unit, system, code }` is not unit-tested.

### 4. Re-slicing semantics

Bracket parsing tolerates `[parent][child]` (`parser.rs:1930`) but `path_resolver::Bracket` does not distinguish a reslice from a chained slice access. Reslice cardinality and `^slicing.discriminator` inheritance from the parent slice are not implemented.

### 5. Invariant `Expression`/`XPath` round-trip into ElementDefinition.constraint

`semantic/invariant.rs` stores Severity/Description/Expression/XPath; `apply_obeys_rule` attaches the invariant key. The actual emission of `ElementDefinition.constraint[].expression` and `.xpath` was not exercised in tests under `crates/maki-core/tests/`.

### 6. `ig.ini` parsing

`crates/maki-core/src/export/file_structure.rs` writes file layout but I could not locate a parser for an existing `ig.ini` (e.g. discovering a project's `template`, `ig`, `usage-stats-opt-out`). SUSHI reads this file. maki's tests under `crates/maki-integration-tests/` indirectly cover it.

---

## Low gaps

### 1. Alias URL-shape lint

There is no lint that an alias's RHS resolves to a syntactically valid URL/OID/UUID. `malformed-alias` (`builtin.rs:135-170`) only checks the LHS shape and `=` placement.

### 2. Comment-only lines after rules

`parser.rs:1420` breaks rule parsing on `CommentLine`, which is correct, but interior `/* */` comments inside an expression list may interrupt token sequences. No regression tests exercised this combination.

### 3. CodeSystem `^designation`, `^property` arrays

Caret-rule machinery exists but the pattern-completion test (`* #code ^designation[+].language = #en`) is not present in the test fixtures under `crates/maki-core/tests/golden_files/`.

### 4. Mapping rule comment + mime-type

`parse_sd_rule_body` (`parser.rs:2785-2799`) consumes `-> "target" "comment"? #lang?`. The optional `#mime-type` on the **language** position is parsed but the FSH spec also allows a comment-only form with mime-type on profile-level mapping rules; corner cases were not exhaustively tested.

---

## Out-of-scope

- **SUSHI-only behaviour** (auto-aliases, `_genonce.sh` output, `_build.sh` output, `--require-valid-slice-name`, package-cache layout). These are tracked in `tasks/sushi_parity_2026-05-09.md`.
- **HL7 IG Publisher integration** — the actual `_genonce`/`_build` invocation is downstream of `maki build` output and out of scope for the language spec.
- **GritQL pattern syntax inside lint rules** — maki extension, not part of FSH.
- **Cross-version extension packages (`hl7.fhir.uv.xver-*`)** — package-resolution layer, not language layer (also tracked in SUSHI parity doc).
- **`maki.yaml` (maki-only config)** — separate from `sushi-config.yaml`, governed by `crates/maki-core/src/config/maki_config.rs`.

---

## Suggested ordering for remediation

1. Indented-rule path inheritance (Critical #2) — touches parser and AST; unblocks correct CodeSystem hierarchies (High #2) and slicing patterns (Critical #1).
2. Slicing `^slicing.discriminator/.rules` propagation into FHIR `ElementDefinition.slicing` (Critical #1 + High #1).
3. In-profile mapping rule handling (Critical #3) — small change in `profile_exporter.rs`, large correctness payoff.
4. Logical default `Parent: Base` and Resource parent-validator (Critical #4) — one-line fix plus a lint rule.
5. `AddCRElement` Logical/Resource arm (High #5).
6. Versioned coding & canonical `|version` lexer node (High #4).
7. Triple-quoted indentation strip (High #6).
8. ValueSet `where` operator vocabulary lint + filter validator (High #7).

Each item has a file and line citation in the matrix above; none requires a redesign of the CST/AST layer.
