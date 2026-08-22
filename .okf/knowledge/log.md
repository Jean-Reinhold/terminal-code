# Terminal Code Knowledge Update Log

## 2026-08-21
* **Creation**: Established an Open Knowledge Format v0.2 bundle for the repository.
* **Inventory**: Recorded the current TypeScript, JavaScript, Bash, web, release-worker, and test surfaces.
* **Planning**: Added an evidence-backed [Rust rewrite plan](plans/rust-rewrite.md), target architecture, compatibility contracts, and parity gates.
* **Limitation**: Recorded the [planning swarm](plans/planning-swarm.md) as blocked because every worker failed authentication before repository access; no worker output was represented as evidence.
* **Harness**: Added an extensive [agentic validation harness](harness/) covering contracts, scenarios, execution, isolation, surfaces, oracles, agents, evidence, CI, security, and H0-H8 delivery.
* **Agent execution**: Routed eight harness planning workstreams to requested DeepSeek V4 Flash tasks; every worker failed in the Anthropic wrapper before the downstream model invocation, so no DeepSeek output was claimed.
* **Implementation**: Added the first deterministic Rust harness vertical: executable C01/C02 catalog, six sandboxed legacy scenarios, exact/differential oracles, SHA-256 evidence, replay, containment/mismatch/corruption tests, and [implementation status](harness/implementation-status.md).
* **H0 completion**: Decomposed C01-C22 into individual contract concepts, mapped all 119 legacy test declarations, replaced the CommonJS target probe with Rust `tode-core`/`tode-contract-probe`, and verified four C02 Rust scenarios against legacy-derived snapshots.
* **H1 completion**: Added explicit sealed run plans, pre-side-effect policy limits, plan-bound evidence roots, plan-owned replay expectations, tamper detection, and fail-closed retry declarations.
* **H2 progress**: Added held TCP/Unix-socket leases, canonical filesystem-tree evidence, file content artifacts, process output budgets, process-group cleanup invariants, and timeout/cleanup tests.
* **H2 completion**: Added canonical filesystem evidence, held port/socket leases, output budgets, process cleanup invariants, bounded Unix JSON-line peers, transcript assertions, timeout/oversize failures, and replayable C05 evidence.
* **H3 C05 parity**: Ported the Unix IPC client to `tode-core` and added Rust success, refusal, timeout, wait, unreadable, framing, and missing-socket evidence without Node test wrappers.
* **Video cleanup**: Removed stale dark/light demo media and the custom client player, browser-verified the cleaned homepage, and added a deferred [certified replacement-video plan](plans/replacement-demo-video.md).
* **H3 C08 parity**: Added the Rust HTTP/1 injector with HTML/CSS/font/header/readiness/error behavior and raw WebSocket upgrade bridging; 7 Rust tests cover all 14 legacy injector cases.
* **H3 C09/C11 parity**: Ported OSC palette parsing/fallbacks and source-preserving JSONC editing/parsing to Rust; 9 new tests raise mapped Rust contract coverage to 22.
* **H3 C10 parity**: Ported complete sRGB/Oklch color math and the full workbench/token theme generator to Rust; 10 tests raise mapped Rust contract coverage to 32.
* **H3 C13 parity**: Ported chord normalization plus Ghostty/Kitty trigger, config, include, emit, and shared-rebind transforms to Rust; 8 tests raise mapped Rust coverage to 40.
* **H3 C01 parity**: Ported exact help/version identity to Rust, switched both C01 scenarios to `tode-contract-cli`, and removed the final Node target manifest from active harness execution.
* **H3 C14 partial**: Ported persisted shortcut claim/import/quit/fallback binding behavior to Rust; full manager-row claimant graph and adversarial convergence remain open.
* **M3 C11 profile ownership**: Added `tode-profile` with exact XDG/install paths, managed/seeded settings precedence, atomic mode-preserving writes, and idempotent installation tests.
* **M3 C12 import**: Ported settings, keybindings, snippets, tasks, extension registry/copy, reports, deduplication, and unsafe/symlink rejection to Rust.
* **M3 C12 service completion**: Added Rust editor discovery, XDG precedence, content summaries, and extension-copy progress; the non-UI import service now has 6 integration tests.
* **M3 theme installation**: Added Rust managed theme extension manifests, registry replacement, old fingerprint cleanup, live-theme output, and idempotent installation.
* **M2 completion**: Ported the final release target/manifest/receipt schemas; all planned pure algorithm/schema families now exist in Rust, with fuzz/mutation expansion reserved for H6.
* **M4 verified artifacts**: Added streamed exact-size/SHA downloads, safe bounded tar.gz extraction, link rejection, failed-download cleanup, and rollback-safe directory swaps in Rust.
* **M4 managed state**: Added typed server state, PID liveness, dual-listener validation, delayed readiness, stale-state cleanup, managed SIGTERM shutdown, and injector-origin behavior in Rust.
* **M4 managed spawn**: Added exact code-server arguments/gallery environment, version capture, process-group spawn, log ownership, readiness, and shutdown integration using a Rust probe.
* **M4 daemon composition**: Composed managed code-server, Rust injector, combined state, warm-up, proxied origin, and complete shutdown into one Rust daemon transaction.
* **M4 terminal-browser resolution**: Added Rust override/vendored/pinned/system-clone precedence, platform Electron layout, exact Bash launcher environment, browser homes, and executable mode.
* **M4 downloaded runtime**: Composed release lookup, exact verified download, strip-one safe extraction, usability validation, tarball cleanup, and launcher creation.
* **M4 daemon command**: Added persistent `tode-daemon` readiness JSON, SIGTERM/Ctrl-C handling, child shutdown, and state cleanup with real Rust-binary integration.
* **M6 basic working CLI**: Added production Rust `tode` help/version/open/shutdown, fallback profile/theme/CSS, persistent daemon start/reuse, terminal-browser launch, and end-to-end Rust-only integration.
* **M6 existing-window reuse**: Added Rust goto/add/diff/new/reuse/wait/review/split/size parsing and production `TODE_IPC` goto/wait/review reuse integration.
* **M6 compatibility flags**: Ported documented ignored flags, value-consuming ignored options, warning-only extension-isolation flags, and strict invalid/missing-value handling.
* **M6 extension management**: Added Rust install/uninstall/list/show-versions parsing, uninstall-first execution, managed profile paths, exit propagation, and Bash-fixture integration.
* **M6 import/theme commands**: Wired production Rust editor discovery/import reports and managed theme installation/fingerprint output with end-to-end profile integration.
* **M6 uninstall**: Added safe Rust owned-path/font/shim/terminal-config uninstall service and production `--uninstall --yes` integration with unrelated-data protection.
* **M6/M7 upgrade**: Added verified Rust current/available/upgraded transactions and production `--upgrade --check` manifest/build selection integration.
