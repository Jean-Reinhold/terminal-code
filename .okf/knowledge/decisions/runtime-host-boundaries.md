---
type: Architecture Decision
title: Runtime Host Boundaries
description: Keep product logic in Rust while bounding host-required JavaScript, WebAssembly loaders, and upstream runtimes.
tags: [adr, rust, wasm, vscode, browser]
status: draft
sources:
  - id: bridge
    resource: ../../../src/bridge.ts
    title: Current generated VS Code extension
  - id: browser
    resource: ../../../src/browser
    title: Current browser host glue
  - id: product
    resource: ../project/product.md
    title: Product composition
---

# Context

The product embeds code-server and terminal-browser. VS Code loads Node extension entry points, browsers load JavaScript/WebAssembly, and a downloader must run before the Rust binary exists. A literal repository and release containing zero JavaScript or shell bytes would remove current features rather than preserve them.

# Decision

“All Rust” means all repository-owned product policy, domain logic, orchestration, state mutation, network services, UI component logic, build/release logic, and native commands are authored in Rust.

Allowed boundaries:

1. Pinned, checksummed upstream terminal-browser and code-server artifacts.
2. Generated JavaScript loader/adapter code required by the VS Code or browser host, with no independent policy and a versioned ABI.
3. Generated WebAssembly loader output from the Rust web build.
4. A minimal POSIX install bootstrap limited to target selection, verified installer download, and `exec` of the Rust installer.

Generated adapters are reproducible artifacts. Their source templates, payload schemas, and tests live with the Rust component that owns the boundary.

# Consequences

* Runtime-host constraints remain visible instead of being mislabeled as fully native.
* Security review can focus on small adapters and pinned upstream artifacts.
* The Rust build must support native plus browser/worker WASM targets.
* Some generated files may contain JavaScript even though no hand-maintained application logic does.

# Rejected Alternatives

* Keep the current TypeScript bridge/browser modules: violates one-source ownership and retains Node build tooling.
* Reimplement code-server or terminal-browser: changes product scope and makes feature parity infeasible.
* Remove embedded web interaction: loses shortcut/import behavior.
* Download an installer without a bootstrap: impossible on a clean machine unless distribution moves entirely to package managers or preselected URLs.

# Acceptance

Before stabilizing this decision, prototype the smallest VS Code adapter and confirm which APIs cannot be called directly from a Rust-generated WebAssembly module. The compatibility contract, not aesthetic language purity, decides the boundary.
