---
type: Compatibility Contract
title: Behavioral Compatibility Matrix
description: Observable behavior that the Rust implementation must retain before clean cutover.
tags: [compatibility, features, acceptance, rust]
status: draft
sources:
  - id: readme
    resource: ../../../README.md
    title: Public CLI and product behavior
  - id: main
    resource: ../../../src/main.ts
    title: Implemented CLI behavior
  - id: tests
    resource: ../../../test
    title: Current regression suite
---

# Rule

A behavior is compatible only when the legacy and Rust implementations produce equivalent observable results from the same isolated fixture. Equivalent does not mean identical internal timing or implementation; it means the same files, protocols, stdout/stderr, exit status, window/editor effect, and visual state where those are contractual.

# Compatibility Matrix

| ID | Contract | Current evidence | Required parity gate |
|---|---|---|---|
| C01 | `--help`, `--version`, default current-directory open, folder/file/new-file targets | `README.md`; `src/main.ts` `HELP`, `main`; `test/target.test.js` | Golden stdout/exit tests plus real filesystem targets |
| C02 | `-g/--goto`, `-a/--add`, `-n/--new-window`, `-r/--reuse-window`, `-w/--wait`, `-d/--diff`, `--review`, `--split`, `--size` | `src/main.ts` `openCommand`; `src/ipc.ts` | Legacy-vs-Rust request and launch fixture matrix, including invalid arity/unknown options |
| C03 | Accepted ignored VS Code flags and warning-only unsupported extension isolation flags | `src/main.ts` `IGNORED`, `IGNORED_WITH_VALUE`, `UNSUPPORTED` | Exact stderr and exit snapshots for each flag/value form |
| C04 | Extension install/uninstall/list/show-version order and code-server profile paths | `src/main.ts` `extensionCommand`, `manageExtensions` | Fake code-server argv/stdio/exit harness |
| C05 | Existing-window reuse through `TODE_IPC`; newline JSON; 4 s default and unbounded `--wait` timeout | `src/ipc.ts`; `src/main.ts` `openCommand` | Unix-socket byte fixtures for success, refusal, timeout, malformed reply, wait completion |
| C06 | Startup ordering: runtime, code-server, terminal palette, font/theme/CSS/settings, shortcuts, bridge, injector, onboarding, browser | `src/main.ts` `openCommand`, `bootEditorUrl` | Event-log comparison with fake process/network/terminal adapters |
| C07 | Pinned terminal-browser/code-server resolution, verified size/SHA-256 download, offline vendored path, readiness, warm-up, shutdown | `src/runtime/release.ts`; `src/codeserver/server.ts` | Local HTTP artifact server and process lifecycle scenarios |
| C08 | HTML CSS injection, content-length correction, uncompressed upstream request, WebSocket upgrade, font route, plain upstream-down error | `test/inject.test.js` | Run the same HTTP/WebSocket fixture suite against Rust injector |
| C09 | Terminal OSC palette queries, variable-width RGB parsing, fallback slots, exact ANSI transfer, live theme propagation | `src/terminal/osc.ts`; theme/live-sync tests | Byte parser corpus plus generated-theme differential fixtures |
| C10 | Theme generation for dark/light/extreme palettes, WCAG AA text contrast, deterministic fingerprint, live theme file and extension | `src/theme/**`; `src/profile.ts`; `test/theme.test.js`, `test/livesync.test.js` | Golden JSON/CSS/hash outputs and live window scenario |
| C11 | Source-preserving JSONC settings edits, managed/seeded precedence, keybinding merge/removal semantics | `src/jsonc.ts`; `src/profile.ts`; theme tests | Byte-level fixture comparison, idempotence, malformed-input behavior |
| C12 | Import settings, keybindings, snippets, tasks, extensions, progress, skipped reasons, first-run flow | `src/import/**`; `test/import.test.js` | Sandboxed compatible-editor fixture trees and report/UI comparisons |
| C13 | Ghostty/Kitty detection, key syntax, default/effective bindings, conflict derivation, moves/unsets/keeps, config includes, signals, undo | `src/shortcuts/**`; `test/shortcuts.test.js` | Backend fixture corpus, process ancestry/signal fakes, byte-idempotent config writes |
| C14 | Shortcut manager convergence: after complete decisions, rerun shows no unresolved conflict and second apply changes no byte | `test/shortcuts-loop.test.js` | Port the closed-loop adversarial driver as a language-neutral acceptance test |
| C15 | Startup review/diff marker is consumed once; theme persists and updates all live sockets; dead sockets are removed | browser glue/live-sync tests | Multi-socket integration tests and single-use marker fixture |
| C16 | Timing command labels/marks and launch-stage output; missing timing is non-error | `src/main.ts` `timingCommand`; browser glue tests | Golden timing output using fixed clock/marks |
| C17 | `--shortcut-setup`, `--import`, `--theme`, `--skill`, `--upgrade`, `--shutdown`, `--uninstall` first-argument dispatch | `README.md`; `src/main.ts` `main` | Command-table black-box suite including trailing arguments and failures |
| C18 | Stable/dev manifests, pinned manifest, installer routes, downloads, GET/HEAD/method errors, range behavior | `release-worker/src/index.ts`; release scripts | HTTP contract suite against legacy worker fixture and Rust WASM worker |
| C19 | Upgrade check/current/available/upgraded/not-install outcomes and atomic swap/receipt | `src/upgrade.ts` | Local release server plus interrupted-download/unpack/swap scenarios |
| C20 | Uninstall confirmation/`--yes`, server stop, runtime/data/state/cache/font/shim/config cleanup | `src/uninstall.ts` | Sandboxed installation tree; exact retained/removed path assertions |
| C21 | Public site layout/content/assets/video/install UX/metadata and `/install` proxy | `web/**`; `web/next.config.ts` | Browser interaction, accessibility tree, responsive visual screenshots, route checks |
| C22 | Embedded import and shortcut pages, token access, apply/cancel/progress behavior | `src/pages/**`; `src/webui/**` | Real browser scenarios against isolated local backend |

# Characteristic Preservation

The following are intentional characteristics, not incidental implementation details:

* JetBrains Mono and terminal-derived palette.
* Fast reuse of an existing editor window through local IPC.
* First open works offline from vendored runtime artifacts.
* User comments and unrelated settings/config lines survive changes.
* Terminal shortcut edits are isolated in a tode-owned include and undo cleanly.
* Existing page layout, responsive behavior, fonts, assets, copy, and interaction timing remain visually recognizable.
* Upgrade and installation replace whole staged trees atomically.

# Freeze Procedure

Before implementing a Rust behavior slice:

1. Give the contract a stable ID from this file.
2. Add missing black-box fixtures around the current implementation.
3. Record normalized golden outputs without machine-specific paths, PIDs, ports, clocks, or network hosts.
4. Run the fixture against the unchanged legacy implementation.
5. Implement Rust until the same fixture passes.
6. Keep the legacy fixture runnable until the clean-cutover gate; do not maintain two independent expected-output sets.
