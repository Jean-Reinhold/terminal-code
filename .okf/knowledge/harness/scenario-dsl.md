---
type: Scenario Specification
title: Versioned Scenario JSONC DSL
description: Safe declarative format for agent-authored and human-reviewed compatibility scenarios.
tags: [harness, scenario, jsonc, schema]
status: draft
sources:
  - id: catalog
    resource: contract-catalog.md
    title: OKF-backed contract catalog
  - id: jsonc
    resource: ../../../src/jsonc.ts
    title: Existing source-preserving JSONC implementation
  - id: protocols
    resource: ../contracts/state-and-protocols.md
    title: Current state and protocols
---

# Decision

Author scenarios as versioned `*.scenario.jsonc`. JSONC is already a product compatibility concern, is straightforward for agents to emit, supports comments for human review, and compiles to canonical JSON. Rust ports the existing source-preserving JSONC behavior before the harness depends on it.

The canonical JSON Schema lives at `harness/schemas/scenario-v1.schema.json`. Unknown fields are rejected. The compiler resolves references, validates capabilities and registry IDs, then emits an immutable `CompiledScenario` without comments or path interpolation ambiguity.

# Example

```jsonc
{
  "$schema": "../../schemas/scenario-v1.schema.json",
  "schema_version": 1,
  "id": "ipc.window-reuse.wait",
  "title": "Wait for a reused window to finish",
  "contracts": ["C02", "C05"],
  "risk": "critical",
  "tags": ["cli", "ipc", "wait"],

  "targets": {
    "mode": "differential",
    "left": "legacy:tode",
    "right": "rust:tode"
  },

  "requires": {
    "os": ["macos", "linux"],
    "capabilities": ["unix-socket", "process-tree", "loopback"],
    "exclusive": [],
    "timeout_ms": 15000
  },

  "sandbox": {
    "fixture": "fixtures/ipc/basic-home",
    "environment": {
      "HOME": { "sandbox_path": "home" },
      "XDG_DATA_HOME": { "sandbox_path": "xdg/data" },
      "XDG_STATE_HOME": { "sandbox_path": "xdg/state" },
      "XDG_CACHE_HOME": { "sandbox_path": "xdg/cache" },
      "TODE_IPC": { "lease": "socket:window" }
    },
    "network": "loopback-only",
    "clock": { "mode": "controlled", "epoch": "2026-01-01T00:00:00Z" }
  },

  "steps": [
    {
      "id": "window",
      "kind": "unix_socket.server",
      "bind": { "lease": "socket:window" },
      "protocol": "json-lines",
      "script": "scripts/ipc/wait-success-v1"
    },
    {
      "id": "open",
      "kind": "process.exec",
      "program": { "target": "tode" },
      "args": ["--wait", { "sandbox_path": "workspace/file.txt" }],
      "stdin": "closed",
      "capture": ["stdout", "stderr", "exit", "process-events"],
      "timeout_ms": 10000
    },
    {
      "id": "request",
      "kind": "unix_socket.await_frame",
      "server": "window",
      "frame": "first",
      "timeout_ms": 2000
    }
  ],

  "observations": [
    { "id": "cli", "kind": "process.result", "from": "open" },
    { "id": "wire", "kind": "unix_socket.transcript", "from": "window" },
    { "id": "tree", "kind": "filesystem.tree", "root": { "sandbox_path": "." } }
  ],

  "normalization": [
    { "observation": "wire", "normalizer": "path.sandbox-root-v1" },
    { "observation": "cli", "normalizer": "process.pid-v1" }
  ],

  "assertions": [
    { "kind": "differential.equal", "observation": "cli" },
    { "kind": "differential.equal", "observation": "wire" },
    { "kind": "json.path", "observation": "wire", "path": "$[0].wait", "equals": true },
    { "kind": "invariant", "rule": "process.no-leaks-v1" }
  ],

  "retry": {
    "max_attempts": 2,
    "only": ["worker-lost", "port-lease-broken"]
  }
}
```

# Top-Level Fields

| Field | Rule |
|---|---|
| `$schema` | Required relative path to reviewed schema |
| `schema_version` | Required integer; compiler supports an explicit range |
| `id` | Globally unique lowercase dotted identifier |
| `title` | Human-readable, non-empty |
| `contracts` | One or more known contract IDs |
| `risk` | Must equal or exceed the highest linked contract risk unless policy grants reviewed exception |
| `targets` | `single`, `differential`, or `metamorphic` mode using target-manifest aliases |
| `requires` | OS/arch/capabilities/exclusive resources/deadline |
| `sandbox` | Fixture, structured environment, network, clock, storage budgets |
| `steps` | Ordered/DAG executable vocabulary with stable IDs |
| `observations` | Explicit evidence requested from steps or sandbox |
| `normalization` | Registry IDs applied to named observations |
| `assertions` | Registry-backed exact/semantic/differential/invariant rules |
| `retry` | Optional infrastructure-only retry policy |

# Safe Value Model

Strings are data, never shell. Paths, leases, target programs, secrets, and artifact references are typed objects. The only supported substitutions are compiled structured values:

* `{ "sandbox_path": "relative/path" }`
* `{ "fixture_path": "relative/path" }`
* `{ "lease": "port:name" }` or `{ "lease": "socket:name" }`
* `{ "target": "tode" }`
* `{ "artifact": "sha256:..." }`
* `{ "secret_handle": "staged-r2-token" }` where policy permits

Absolute paths, `..` traversal, unregistered environment inheritance, and implicit string interpolation are rejected.

# Step Vocabulary

Initial reviewed kinds:

* filesystem: `fs.mkdir`, `fs.write`, `fs.copy_fixture`, `fs.remove`, `fs.chmod`, `fs.snapshot`;
* process: `process.exec`, `process.spawn`, `process.signal`, `process.wait`, `process.assert_running`;
* PTY/terminal: `pty.spawn`, `pty.send_bytes`, `pty.await_bytes`, `terminal.osc_reply`, `terminal.capture`;
* Unix socket: `unix_socket.server`, `unix_socket.await_frame`, `unix_socket.send_frame`, `unix_socket.close`;
* network: `http.server`, `http.request`, `websocket.client`, `network.fault`;
* browser: `browser.open`, `browser.action`, `browser.await`, `browser.capture`;
* clock/fault: `clock.advance`, `fault.arm`, `fault.release`;
* release: `release.stage_r2`, `release.publish_manifest`, `release.install`, `release.upgrade`, `release.rollback`.

A step names a built-in implementation and typed parameters. `shell`, `script`, `eval`, inline JavaScript, inline Rust, and arbitrary URL fetch are forbidden. The example's `script` is a registry ID for a compiled deterministic peer state machine, not a file or executable script.

# Observation Vocabulary

* process result/events/resource summary;
* stdout/stderr raw bytes and decoded text;
* filesystem tree, bytes, metadata, xattrs where relevant;
* JSON/JSONL frames and raw transport bytes;
* OSC/ANSI/PTY transcript;
* HTTP request/response, headers, body, range/upgrade events;
* browser screenshot, DOM snapshot, accessibility tree, console/network log;
* timing marks against controlled/monotonic clocks;
* release manifest/archive tree/checksum/install receipt;
* sandbox containment and leak report.

Each observation records producer adapter, schema version, content digest, byte length, redaction state, and source step.

# Assertions

Assertions are side-effect free and registry-backed:

* exact bytes/text/JSON/tree/metadata;
* semantic JSON/HTTP/process equivalence;
* legacy-vs-Rust differential equality;
* invariant rule;
* metamorphic relation across scenario variants;
* bounded numeric/timing relation where the contract defines tolerance;
* visual comparison plus DOM/accessibility invariants;
* expected controlled failure.

Unknown or unavailable assertions yield `scenario_invalid` or `inconclusive`; they never pass by omission.

# Normalizers

Scenarios select reviewed normalizer IDs only. Normalizers declare input/output schemas, exact fields changed, and whether they are permitted for each contract/risk. A normalizer cannot delete stderr, exit status, ordering, unknown JSON fields, HTTP headers, or filesystem paths without explicit policy.

# Fixtures and Baselines

References resolve within `harness/fixtures` or `harness/baselines`, must match committed digests, and are copied read-only into a scenario seed area before each target receives its own writable clone. Agents can propose new bytes under a proposal artifact; only reviewed digests become fixtures/baselines.

# Version Evolution

Minor additive schema changes keep the same `schema_version` only when old compilers safely reject/ignore nothing. Any semantic or field-removal change increments the integer and ships a deterministic migration plus before/after digest report. CI parses every retained scenario with the oldest supported compiler before dropping support.
