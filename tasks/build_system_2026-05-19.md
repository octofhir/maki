# Maki Build System Expansion — 2026-05-19

## Current State

Maki already ships a SUSHI-compatible build pipeline. The relevant
modules:

- `crates/maki-cli/src/commands/build.rs` — CLI entry point.
- `crates/maki-core/src/export/build.rs` — `BuildOrchestrator`.
- `crates/maki-core/src/export/{profile,extension,valueset,codesystem,instance,logical,mapping}_exporter.rs` — per-resource exporters.
- `crates/maki-core/src/export/{differential_generator,snapshot,invariant_processor,ruleset_integration}.rs`.
- `crates/maki-core/src/export/{ig_generator,menu_generator,package_json,predefined_resources,file_structure,build_cache}.rs`.
- `crates/maki-core/src/canonical/mod.rs` — package install / lookup,
  already uses `tar` + `flate2`.
- Lint, format, autofix, diagnostics (incl. `sushi-ignoreErrors.txt`),
  rules engine, semantic model, lossless CST — all in place.

So this task is not "implement a build system." It is **bolt a plugin
boundary, a watch/incremental engine, multi-target builds, an FHIR NPM
tarball emitter and an HTML site renderer onto the existing
orchestrator**, without disturbing the SUSHI-compatible behaviour
people already rely on.

## Goal

Turn the existing build pipeline into a composable, watch-driven,
multi-target IG build system:

1. Extract a stable plugin trait + lifecycle on top of
   `BuildOrchestrator`. Every existing exporter becomes an internal
   built-in plugin.
2. Add a watch / dev mode that does incremental rebuilds via a
   resource graph keyed by canonical URL.
3. Add multi-target builds (R4 / R4B / R5 from one source tree).
4. Add an FHIR NPM tarball emitter (`package.tgz`).
5. Add a browsable HTML site renderer.

## Non-goals

- Authoring profiles in any language other than FSH / JSON / YAML.
  Other source DSLs are explicitly out of scope.
- Sandboxed third-party plugins (WASM, dylib, scripting). v1 plugins
  are Rust crates linked statically. Revisit only after the trait API
  has been stable for a release.
- Theming for the HTML site. v1 ships one default theme.
- Rewriting any existing exporter. The plugin API wraps them; the
  exporters keep their current shape.

## Hard Constraint: SUSHI Drop-in Behaviour Must Not Regress

Everything in this plan is **opt-in**. The default invocation
(`maki build`, no flags, no `maki.toml` extras) must keep producing
exactly the same artefacts in exactly the same layout as today, byte-
for-byte where SUSHI parity tests already lock it in.

Concretely:

- The default plugin chain is the current hard-coded pipeline,
  rewritten 1:1 as plugin calls. No plugin is dropped, no plugin order
  changes, no new pass is added to the default chain.
- New features land behind explicit flags / config keys, all
  defaulting to off:
  - `maki dev` is a separate command; `maki build` never starts a
    watcher.
  - Multi-target only activates when `[[targets]]` is present in the
    config. Single-target builds keep the legacy `fsh-generated/`
    output layout untouched.
  - NPM `.tgz` emission is gated on a `--package-tgz` flag (or a
    config opt-in). Today's loose `fsh-generated/` tree is still the
    default.
  - HTML site emission is gated on a `--site` flag (or config
    opt-in). Default build emits no site.
  - Plugin loading from config is additive only; users cannot
    accidentally disable a built-in plugin without an explicit
    `disable = [...]` list.
- Every phase ships with a regression gate: full golden-file diff of
  `dist/` for the example IGs, run on CI. A diff is a release blocker
  unless the SUSHI reference output moved first.
- The plugin trait lives behind `#[non_exhaustive]` and a
  `maki-core` minor-version bump only; no public-API breakage in
  patch releases.

If a refactor cannot meet these guarantees, the refactor is wrong, not
the guarantee.

## Architectural Principles

1. **Lossless CST stays the source of truth.** Plugins consume the
   typed AST and semantic model; they do not re-parse FSH.
2. **Resource graph keyed by canonical URL.** Cross-references between
   resources are canonical-URL edges. The graph drives validation,
   ordering, incremental rebuilds.
3. **Plugin trait API, statically linked.** Hooks follow a
   Rollup/Vite-style lifecycle. The built-in pipeline is expressed as
   the default plugin chain.
4. **Incremental by default in dev mode.** Watch mode rebuilds only the
   reverse closure of changed resources. One-shot builds rebuild
   everything; reuse `build_cache.rs` where applicable.
5. **Multi-target via config matrix.** One source tree → N artefacts
   per FHIR version / feature flag, run in parallel.

## Workspace Layout

No new top-level crates yet. The new code lives inside existing
crates so the SUSHI parity work stays in one place:

- `maki-core::plugin` — `Plugin` trait, hook context types, source
  map, diagnostics sink. Public re-export from `maki-core`.
- `maki-core::export::orchestrator` — existing orchestrator,
  refactored to drive a plugin chain instead of hard-coded calls.
- `maki-core::watch` — file watcher + reverse-deps engine.
- `maki-core::export::npm_tarball` — FHIR NPM `package.tgz` emitter.
- `maki-core::export::site` — browsable HTML site renderer.
- `maki-cli::commands::dev` — `maki dev` watch loop.

Built-in plugins live next to the modules they wrap:
`fsh_loader`, `json_loader`, `yaml_loader`, `snapshot_plugin`,
`narrative_plugin`, `validate_plugin`, `ig_resource_plugin`,
`npm_plugin`, `site_plugin`. Each is just a `struct Foo;
impl Plugin for Foo`.

Carve them into their own crates (`maki-plugin-*`) only after the
trait API is stable.

## CLI Surface

Already present, gets extended:

- `maki build` — one-shot build. Add `-t / --target <name>` flag.
- `maki dev` — watch loop, incremental, dev HTTP server. **New.**
- `maki info` — resolved config, plugin chain, target matrix. **New.**
- `maki validate` — validation-only pass, no emit. **New.**

`lint`, `format`, `rules`, `config`, `init` stay as they are.

## Plugin Trait Sketch

```rust
pub trait Plugin: Send + Sync {
    fn name(&self) -> &'static str;

    fn build_start(&self, _ctx: &mut BuildCtx) -> Result<()> { Ok(()) }

    fn load_source(&self, _ctx: &mut LoadCtx, _path: &Path)
        -> Result<Option<Vec<Resource>>> { Ok(None) }

    fn transform(&self, _ctx: &mut TransformCtx, _res: &mut Resource)
        -> Result<()> { Ok(()) }

    fn before_snapshot(&self, _ctx: &mut SnapshotCtx) -> Result<()> { Ok(()) }
    fn after_snapshot (&self, _ctx: &mut SnapshotCtx) -> Result<()> { Ok(()) }

    fn before_validate(&self, _ctx: &mut ValidateCtx) -> Result<()> { Ok(()) }
    fn after_validate (&self, _ctx: &mut ValidateCtx) -> Result<()> { Ok(()) }

    fn generate_bundle(&self, _ctx: &mut BundleCtx) -> Result<()> { Ok(()) }
    fn write_bundle  (&self, _ctx: &mut BundleCtx) -> Result<()> { Ok(()) }

    fn handle_hot_update(&self, _ctx: &mut HotUpdateCtx, _changed: &Path)
        -> Result<HotUpdateResult> { Ok(HotUpdateResult::Propagate) }

    fn build_end(&self, _ctx: &mut BuildCtx) -> Result<()> { Ok(()) }
}
```

`*Ctx` structs expose: resource graph handle, current target,
diagnostics sink, file→resource source map, plugin-local cache slot.
The default chain is: `fsh_loader`, `json_loader`, `yaml_loader`,
`snapshot_plugin`, `narrative_plugin`, `validate_plugin`,
`ig_resource_plugin`, `npm_plugin`, `site_plugin`.

## Resource Graph

Five edge types, all keyed by canonical URL:

1. `canonical_ref` — generic Reference / Reference target.
2. `meta_profile` — `Resource.meta.profile[]`.
3. `binding` — `ElementDefinition.binding.valueSet`.
4. `vs_to_cs`  — ValueSet → CodeSystem (`compose.include.system`).
5. `package_dep` — IG dependency on another package.

Graph is rebuilt lazily on access. Transforms append edges via the
`TransformCtx` API. Reverse closure over edges drives
`handle_hot_update` and watch mode invalidation.

## Incremental / Watch Mode

- Loader records `file → resource_id[]` in the source map.
- Transform records `resource → canonical_urls_touched[]`.
- On file change:
  1. Re-run the affected file's loader.
  2. Diff produced resources against the previous snapshot.
  3. Walk reverse-deps closure; re-run `transform` and `validate`
     only on dirty resources.
  4. Re-emit bundles whose contents changed.
- File watcher: `notify` crate. Debounce 50 ms.
- Start with hand-rolled memoisation. Move to `salsa` only if hot loops
  show cache-key overhead in the bench suite.

## Multi-target Builds

Config:

```toml
# maki.toml
[[targets]]
name = "r4"
fhir = "4.0.1"

[[targets]]
name = "r5"
fhir = "5.0.0"
```

Conditional source content via a `when()` preprocessor:

- JSON / YAML: `$when` key, e.g. `{ "$when": "fhir.gte(5.0)", "field": … }`.
- FSH: `^fhirVersion` and existing RuleSet conditional machinery, plus
  a new `When(fhir.gte("5.0"))` directive. Final syntax decided in
  the phase-5 design spike.

Targets run in parallel via `rayon`. Each produces `dist/<target>/`.

## NPM Tarball Emitter

- USTAR tar via the existing `tar` crate (already a transitive dep
  through canonical-manager).
- gzip via `flate2`.
- Layout matches the FHIR NPM Package spec: `package/package.json`,
  `package/*.json` for each resource, `package/.index.json`.
- Reuse `export/package_json.rs` for `package.json` generation; only
  new code is the file-tree → tarball walker.

## HTML Site Renderer

- Sidebar nav from the resource graph + `menu_generator.rs`.
- Per-resource pages (one `.html` per Profile / Extension / VS / CS /
  Instance / Logical / Mapping).
- Markdown landing page from `input/pagecontent/index.md` (matches
  IG Publisher convention).
- Templates via `maud` (compile-time) for speed and zero-string-escape
  bugs.
- Static output under `dist/<target>/site/`.

## Phased Roadmap

### Phase 1 — Plugin Boundary (2 weeks)
- Design spike: plugin trait final shape, ctx structs, error model.
- Refactor `BuildOrchestrator` to drive a plugin chain. The current
  hard-coded pipeline becomes the default chain (`fsh_loader` →
  `…` → `npm_plugin`). External behaviour unchanged.
- `maki info` reports resolved plugin chain.

### Phase 2 — Loaders + Resource Graph (2 weeks)
- `json_loader`, `yaml_loader` plugins.
- Build the canonical-URL-keyed resource graph during the loader pass.
- Move existing cross-reference checks into a `validate_plugin`.
- `maki validate` CLI command.

### Phase 3 — Watch Mode (2 weeks)
- `notify` watcher wired into the plugin runner.
- File→resource source map; reverse-deps closure.
- `handle_hot_update` dispatch.
- Dev HTTP server (`axum`) with WebSocket reload channel.
- Benchmark target: edit one file in a 500-resource IG, rebuild
  under 100 ms wall-clock.

### Phase 4 — NPM Tarball + HTML Site (2 weeks)
- `npm_plugin` writes `package.tgz`.
- `site_plugin` writes the browsable HTML tree.
- Golden-file tests vs. SUSHI tarball output (modulo timestamps and
  IG Publisher narrative diffs).

### Phase 5 — Multi-target (1 week)
- `targets:` config schema.
- `when()` preprocessor across JSON / YAML / FSH.
- Parallel target execution via `rayon`.
- Per-target `dist/<name>/` output trees.

### Phase 6 — Polish (1 week)
- SARIF + GitHub Actions diagnostic formats threaded through the
  build pipeline (already exist for lint; reuse).
- Plugin loading from config (`[plugins]` block) — for now this only
  toggles built-in plugins; external plugin loading stays deferred.
- Docs: plugin authoring guide, hook reference, examples.
- Bench suite: vs. SUSHI on US Core, mCODE, IPS.

## Dependencies (new)

| Crate | Use |
| --- | --- |
| `notify`        | file watching |
| `axum`          | dev HTTP server |
| `tokio-tungstenite` | WebSocket reload channel |
| `rayon`         | parallel target builds |
| `maud`          | HTML site templates |

`tar` and `flate2` are already transitive deps via canonical-manager;
promote them to direct deps when the NPM emitter lands.

## Open Questions

1. Plugin config schema — TOML-only, or also expose a Rust builder API
   for callers embedding maki as a library?
2. `when()` conditional FSH — new directive vs. repurposed RuleSet
   machinery. Decide in the phase-5 spike.
3. Snapshot generation — the existing `snapshot.rs` handles most
   Profile / Extension cases. Audit which SDs still need a fallback
   to `validator.jar`; ideally none in the medium term.
4. Cross-source canonical resolution — FSH-, JSON- and YAML-defined
   resources must share one symbol table. Audit `maki-core::semantic`
   for assumptions tied to CST origin.
5. `salsa` vs hand-rolled memoisation — defer until phase 3 benches
   show actual cost.

## Success Criteria

- **Drop-in parity preserved.** Default `maki build` output on every
  example IG matches the pre-refactor `dist/` byte-for-byte (modulo
  timestamps). This is the gating criterion; every other criterion is
  secondary.
- `maki build --package-tgz` produces FHIR NPM tarballs byte-identical
  to SUSHI's on the existing examples directory (modulo timestamps).
- `maki dev` rebuilds a single-file edit on US Core (~500 resources)
  under 100 ms wall-clock.
- An external author can write a plugin in <100 lines of Rust against
  the trait API and have it loaded via config.
- Cold full build on US Core is at least 5× faster than SUSHI +
  IG Publisher combined.
