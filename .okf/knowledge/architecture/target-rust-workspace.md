---
type: Target Architecture
title: Target Rust Workspace
description: Proposed Cargo workspace and dependency rules for the clean-cutover system.
tags: [architecture, rust, cargo, target-state]
status: draft
sources:
  - id: current
    resource: current-system.md
    title: Current system architecture
  - id: constraints
    resource: ../project/rewrite-constraints.md
    title: Rust rewrite constraints
---

# Design Principles

* One Cargo workspace, one locked dependency graph, and one Rust formatting/lint/test policy.
* Protocol and persisted-state types are leaf dependencies; orchestration depends inward, never the reverse.
* Filesystem, process, network, terminal, and clock effects sit behind narrow concrete adapters only where deterministic tests need substitution.
* Preserve exact external behavior before improving internals.
* Keep crates large enough to own coherent state; do not create a crate per file or command.

# Proposed Repository Shape

```text
Cargo.toml
Cargo.lock
rust-toolchain.toml
crates/
  tode-cli/             # native `tode` binary and compatibility parser
  tode-core/            # command use cases and launch orchestration
  tode-protocol/        # wire/state/release manifest types; minimal dependencies
  tode-runtime/         # processes, code-server, injector, runtime fetch/update
  tode-profile/         # XDG paths, JSONC, import, settings, keybindings, themes
  tode-shortcuts/       # conflict model plus Ghostty/Kitty adapters
  tode-web/             # embedded UIs and public-site Rust/WASM/static output
  tode-release-worker/  # Cloudflare Worker compiled to WebAssembly
  tode-harness/         # deterministic scenarios, sandbox, adapters, oracles, evidence
  tode-harness-agent/   # provider-neutral agent roles, proposals, provenance
  xtask/                # keymap generation, dist, release, checksums, publishing
assets/                 # fonts, logos, keymaps, CSS, media
harness/                # schemas, policies, scenarios, fixtures, baselines, prompts
compat/                 # temporary frozen legacy target; deleted at clean cutover
web-static/             # generated public-site output; never hand-edited
```

# Crate Responsibilities

| Crate | Owns | Likely dependencies |
|---|---|---|
| `tode-protocol` | `OpenRequest`, replies, server state, release manifests, theme/palette DTOs, newline framing | `serde`, `serde_json` |
| `tode-profile` | Exact XDG path rules, atomic state files, source-preserving JSONC patching, editor discovery/import, theme/color generation | `serde`, `sha2`; port current JSONC and color algorithms until differential tests permit replacement |
| `tode-shortcuts` | Chord normalization, decisions, holds, convergence, Ghostty/Kitty parsing/config/reload | `tode-profile`, `serde`, `nix` |
| `tode-runtime` | terminal-browser/code-server resolution, verified downloads, unpacking, process groups, readiness, HTTP/WebSocket injection, upgrade swap | `tokio`, `reqwest` with rustls, `hyper`, `sha2`, archive crates, `nix` |
| `tode-core` | Command use cases, launch state machine, onboarding, extension operations, shutdown/uninstall coordination | protocol/profile/shortcuts/runtime |
| `tode-cli` | Exact current help text, low-level compatibility parsing, stdio and exit mapping | `lexopt`, `tode-core`, `tracing-subscriber` |
| `tode-web` | Shared Rust components for public and embedded pages; generated static output and WASM interaction | `leptos`, `wasm-bindgen` through build output only |
| `tode-release-worker` | Existing GET/HEAD routes, manifest enrichment, R2 streaming/ranges | Cloudflare `worker`, `serde`, `tode-protocol` with a WASM-safe feature set |
| `tode-harness` | OKF catalog compiler, scenario/run model, scheduler, sandbox, adapters, observations, normalizers, oracles, artifacts, reports, replay, certification | `serde`, schema validation, ported JSONC, `tokio`, `nix`, `tempfile`, `sha2`, surface-specific clients; no model SDKs |
| `tode-harness-agent` | Provider adapters, structured role DAGs, redaction, proposal admission, provenance cache, adversarial and evidence review | public `tode-harness` models plus provider clients behind features |
| `xtask` | Versioning, pinned keymaps, artifact layout, checksums, manifests, local dist, release publication | `xshell`, `cargo_metadata`, `sha2`, archive crates |

Dependencies are candidates, not approvals. Pin choices only after maintenance, license, MSRV, WASM, and supply-chain review.

# Dependency Direction

```text
tode-cli -> tode-core -> {tode-runtime, tode-profile, tode-shortcuts}
                         tode-shortcuts -> tode-profile
{tode-runtime, tode-profile, tode-shortcuts} -> tode-protocol
tode-web -> domain DTOs/use-case ports, never native process adapters
tode-release-worker -> tode-protocol only
xtask -> release manifest types, never application orchestration
tode-harness-agent -> tode-harness public models; never the reverse
xtask -> tode-harness public CLI/library for gates; never duplicate execution/oracles
```

Cycles are prohibited. `tode-profile` must not depend on `tode-shortcuts`; move shortcut-owned binding policy out of the current profile module.

The harness trust boundary is architectural: deterministic execution and verdict code in `tode-harness` has no model-provider dependency. Agents can broaden plans and review evidence through `tode-harness-agent` but cannot mutate approved contracts, baselines, policy, evidence, or verdicts. See the [harness architecture](../harness/architecture.md) and [deterministic-agent decision](../decisions/deterministic-agent-boundary.md).

# Runtime Model

Use one Tokio runtime in native binaries. Model launch and shutdown as explicit state transitions rather than global promises/caches. Own child process groups and reap them. Use bounded readiness deadlines and typed failures while mapping messages and exit statuses at the CLI boundary.

All state-changing operations follow: read and validate, compute complete replacement, write to a sibling temporary file, flush where durability matters, atomically rename, then reload. Preserve current config comments and unrelated bytes.

# Host Boundaries

* The generated VS Code extension needs a small JavaScript host adapter because VS Code loads Node extensions. Generate it from a versioned template and keep policy in Rust/protocol data.
* Browser UI logic is authored in Rust and compiled to WebAssembly. Generated `wasm-bindgen` loader code is build output, not a second implementation.
* terminal-browser and code-server remain pinned, verified upstream artifacts.
* The public installer may retain a minimal POSIX bootstrap whose only job is target selection, verified download, and execution of a Rust installer.

See [runtime-host decision](../decisions/runtime-host-boundaries.md).

# Build Profiles

* Native release binaries: LTO, stripped symbols in artifacts, deterministic version metadata.
* Worker/web WASM: separate target jobs; do not make native workspace checks depend on a browser target.
* `default-members` cover native crates. CI explicitly checks every excluded WASM member.
* Release artifacts remain platform-specific and retain the existing manifest fields and URL layout.
