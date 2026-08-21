---
type: Adapter Specification
title: Validation Surface Adapters
description: Deterministic adapters and observations for every terminal-code external surface.
tags: [harness, adapters, cli, browser, terminal]
status: draft
sources:
  - id: contracts
    resource: ../contracts/compatibility.md
    title: C01-C22 compatibility matrix
  - id: protocols
    resource: ../contracts/state-and-protocols.md
    title: State and protocol contracts
  - id: current
    resource: ../architecture/current-system.md
    title: Current architecture
---

# Adapter Contract

Each adapter has a stable ID/version and implements a conceptual lifecycle:

```rust
trait HarnessAdapter {
    fn validate(&self, step: &CompiledStep, capabilities: &Capabilities) -> Result<()>;
    async fn provision(&mut self, sandbox: &Sandbox) -> Result<AdapterHandle>;
    async fn execute(&mut self, step: &CompiledStep, ctx: &StepContext) -> StepOutcome;
    async fn observe(&mut self, request: &ObservationRequest) -> Result<ObservationRef>;
    async fn teardown(&mut self, reason: TeardownReason) -> TeardownEvidence;
}
```

Actual traits should avoid object allocation where static dispatch and enum registries are clearer. The invariant matters: validation and containment occur before execution, observations are typed, and teardown always returns evidence.

# Surface Inventory

| Adapter | Executes/captures | Primary contracts |
|---|---|---|
| CLI/process | argv, stdin, stdout/stderr bytes, exit/signal, duration, descendants | C01-C04, C16-C17, C20 |
| Filesystem/state | tree, bytes, modes, links, xattrs, atomicity, idempotence, containment | C10-C14, C19-C20 |
| Unix socket/IPC | bind/connect, raw frames, JSONL messages, timing, close/error | C05, C15 |
| HTTP/WebSocket | requests, upstream behavior, headers/body, range/HEAD, upgrade frames, disconnects | C08, C18-C19 |
| Runtime/process lifecycle | ports, readiness, warm-up, process state, PID/stale state, shutdown/leaks | C06-C07, C15-C17 |
| PTY/OSC/ANSI | terminal query/reply bytes, input, output, screen/transcript semantics | C09-C10, C13-C14 |
| Terminal backend | Ghostty/Kitty keymaps/config include/reload/ancestry and real smoke behavior | C13-C14 |
| Browser/UI | routes, actions, DOM, accessibility, screenshot, console/network, storage | C15, C21-C22 |
| Release/archive/install | archive entries/modes, manifest, checksum, staged tree, receipts, rollback | C18-C20 |
| Worker/R2 | immutable objects, manifest reads/writes, HTTP streaming/ranges/cache behavior | C18-C19 |

# CLI and Process Adapter

* Program comes from a target manifest alias plus verified digest; no raw executable path from agent-authored data.
* Arguments are a structured list; no shell expansion.
* Captures raw stdout/stderr separately and decoded UTF-8 with invalid-byte annotations.
* Records exit code or signal, process-tree events, cwd, environment-key allowlist, and timeout source.
* Supports piped/closed/inherited-from-PTY stdin only; host terminal inheritance is forbidden in automated runs.
* Uses fixed terminal width/color policy when output is presentation-sensitive.

# Filesystem Adapter

Snapshots canonical relative paths with entry kind, mode, size, content digest, symlink target, and optional platform metadata. Raw content is separate content-addressed evidence. Comparison policies declare whether mtime/uid/xattr matter.

Atomicity faults arm deterministic checkpoints around write/fsync/rename when using test-aware Rust code. Black-box legacy faults use filesystem/proxy/VM mechanisms and record weaker guarantees.

JSON/JSONC/config observations include raw bytes plus parsed semantic form; byte parity and semantic parity are separate assertions.

# IPC Adapter

Provides short leased Unix sockets and deterministic peer state machines. Captures raw transport bytes, frame boundaries, parsed JSON, connection timing, half-close/reset behavior, and peer script transitions. It can delay/refuse/malformed-reply/close at reviewed checkpoints.

No default normalizer may reorder JSON arrays, add omitted fields, or hide timeout/error text.

# HTTP and WebSocket Adapter

Uses local origin aliases from the network registry. Captures request method/path/query, ordered duplicate headers where observable, body bytes, response status/headers/body, streaming chunks, content length, compression, range, ETag/cache, connection close, and WebSocket upgrade/frames.

Peer state machines cover slow readiness, upstream unavailable, truncated artifacts, wrong size/hash, redirect policy, range requests, and midstream disconnects.

# Runtime Adapter

Owns fake/real code-server and terminal-browser target manifests, readiness probes, process groups, port leases, and state-file fixtures. Event traces have stable event kinds such as `spawned`, `listening`, `probe`, `ready`, `warmup`, `shutdown_requested`, `exited`, and `killed`; PIDs/ports are values normalized only in rendered comparisons.

A real smoke target is distinct from a deterministic fake target. Reports never imply a fake process proves upstream integration.

# PTY and OSC Adapter

Captures raw byte streams first, then derives ANSI/OSC events and an optional screen model. Scripts can answer palette queries with variable component widths, BEL/ST terminators, partial slots, delayed chunks, interleaved input, or no response. Timing uses controlled idle/hard-cap policies.

Screen snapshots supplement raw transcripts; they do not replace byte assertions for protocol contracts.

# Ghostty and Kitty Adapters

Fixture mode supplies captured/default keymaps and process ancestry without invoking user binaries. Real mode launches pinned/recorded versions with isolated config roots and verifies actual config load/reload.

Observations include effective keymap input, parsed binds/docs, tode-owned include bytes, parent config bytes, decisions, reload signal target, post-reload effective bindings, undo bytes, and closed-loop convergence.

# Browser Adapter

Drive a real browser through a Rust CDP or WebDriver implementation selected by an implementation spike. Required capabilities include navigation, resilient role/test-ID selectors, keyboard/pointer actions, network interception, console capture, DOM serialization, accessibility tree, screenshot, viewport/device scale, storage/profile isolation, and tracing.

Selector priority: explicit stable test IDs for application controls, accessible role/name, then reviewed CSS. Pixel coordinates are reserved for terminal-browser/hardware scenarios.

Browser observations:

* screenshot in lossless format;
* normalized DOM with volatile attributes policy;
* accessibility tree and focus order;
* console errors/warnings;
* request/response ledger;
* URL/history/storage changes;
* measured interaction/readiness marks.

# Release and R2 Adapter

A local staged service implements R2 object semantics and records object key/digest/metadata, immutable writes, latest-pointer compare-and-swap, ranges/HEAD, and failure injection. Staged external deployment uses separate credentials/namespace and never production latest.

Archive observation streams entries without extracting first, validates safety/budgets, then compares canonical entry order-independent trees while preserving per-file bytes/modes.

# Adapter Quality Gates

Every adapter must have self-tests proving:

* invalid/uncontained input is rejected before side effects;
* each advertised observation can detect a deliberate mismatch;
* teardown removes all owned resources after success/failure/timeout/cancel;
* output schema is stable and versioned;
* redaction cannot convert a relevant mismatch into equality;
* unsupported capability yields inconclusive rather than a weaker silent path;
* adapter upgrade runs retained observation fixtures for backward compatibility.
