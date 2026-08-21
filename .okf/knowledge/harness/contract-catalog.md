---
type: Contract Model
title: OKF-Backed Contract Catalog
description: Machine-compilable compatibility concepts, coverage graph, risk policy, and change-impact selection.
tags: [harness, contracts, okf, coverage]
status: draft
sources:
  - id: matrix
    resource: ../contracts/compatibility.md
    title: Current C01-C22 compatibility matrix
  - id: spec
    resource: ../index.md
    title: OKF v0.2 bundle root
---

# Source of Truth

Each compatibility contract becomes one OKF concept under `.okf/knowledge/contracts/features/`. The existing matrix remains the human index during migration, then becomes a generated summary. The harness compiles concept frontmatter into a canonical catalog; it never edits the catalog directly.

Example concept frontmatter:

```yaml
---
type: Compatibility Contract
title: Existing-window IPC reuse
contract_id: C05
description: Reuse a running editor through the TODE_IPC Unix socket.
tags: [ipc, cli, wait]
status: stable
risk: critical
owners: [runtime]
surfaces: [cli, unix-socket, process]
source_paths:
  - src/ipc.ts
  - src/main.ts
symbols:
  - src.ipc.sendToExtension
scenario_ids:
  - ipc.window-reuse.success
  - ipc.window-reuse.wait
  - ipc.window-reuse.errors
platforms: [macos, linux]
stale_after: 2026-11-21T00:00:00Z
sources:
  - { id: implementation, resource: ../../../../src/ipc.ts }
  - { id: tests, resource: ../../../../test/livesync.test.js }
---
```

Harness-defined extension fields are permitted by OKF v0.2. Required harness fields are enforced by the catalog compiler, not the generic OKF specification.

# Catalog Compiler

`tode-harness catalog check` performs:

1. Discover all concepts with `type: Compatibility Contract`.
2. Parse and schema-check harness fields.
3. Reject duplicate/malformed `contract_id`, unknown risk/surface/platform, broken paths/symbols/scenarios, and stable concepts without owners/evidence.
4. Load scenario metadata and verify reciprocal contract links.
5. Build graph edges among contract, source path, symbol, scenario, fixture, baseline, surface, platform, owner, decision, and historical run.
6. Calculate coverage and staleness policy.
7. Emit canonical sorted `catalog.json` plus its SHA-256.

The emitted catalog is a build artifact. CI checks regeneration produces no diff but does not commit generated data unless a separate consumer requires it.

# Coverage Graph

```text
SourceFile/Symbol -> implements -> Contract
Contract -> verified_by -> Scenario
Scenario -> consumes -> Fixture/Baseline
Scenario -> observes -> Surface
Scenario -> requires -> Capability/Platform
Contract -> governed_by -> Decision
Run -> executed -> Scenario
Verdict -> supports/refutes -> Contract
AgentTask -> proposed/reviewed -> Scenario/Verdict
```

Coverage is a graph query, not a line percentage. Line/branch coverage can help find dead test regions but cannot prove compatibility.

# Risk Tiers

| Tier | Examples | Minimum coverage |
|---|---|---|
| Critical | user-state mutation, upgrade/install, release publication, IPC wait, terminal config | exact/differential scenarios, fault cases, platform scenarios, adversarial review, mutation evidence |
| High | runtime lifecycle, injector, bridge, import, shortcut state machine | differential/invariant scenarios, malformed inputs, process/network failures, nightly mutation/fuzz |
| Medium | CLI presentation, timing reports, static web interactions | exact/semantic plus representative browser/platform coverage |
| Low | non-contractual diagnostics or internal reports | deterministic unit/integration checks and mapping |

Agents may propose raising risk. Lowering risk requires a reviewed OKF change with evidence.

# Impact Selection

The deterministic selector takes the union of:

* contracts naming changed source paths/symbols;
* inbound/outbound dependency-graph impact up to a policy depth;
* scenarios linked to changed fixtures/baselines/normalizers/adapters;
* all critical sentinels for affected deployables;
* historical scenarios that co-failed with selected scenarios;
* mandatory cross-cutover parity smoke scenarios.

Agent impact reviewers receive the deterministic set and may add contracts/scenarios or report an unmapped behavior. They cannot remove entries. Disagreements are retained in the run plan.

# Staleness

A contract is stale when any configured condition holds:

* `stale_after` has passed;
* cited implementation path/symbol disappeared or changed without scenario evidence;
* scenario or baseline digest changed after latest verification;
* pinned upstream code-server/terminal-browser/terminal version changed;
* no successful required-platform run exists inside the freshness window;
* an accepted decision changed the contract boundary.

Stale critical/high contracts fail certification. A planning agent can propose refresh work but cannot clear staleness.

# Ownership and Review

Each stable contract has an owning domain and review policy. Changes to behavior require:

1. explicit contract diff;
2. reason and compatibility impact;
3. updated scenarios/baselines;
4. legacy/Rust evidence while the oracle exists;
5. designated reviewer approval;
6. bundle log entry.

An implementation diff that changes observations without an approved contract change remains a failure, even if an agent calls the new behavior better.

# Migration of C01-C22

M0/H0 converts the matrix into 22 individual concepts without changing semantics. The matrix and per-contract concepts are checked for ID/title/summary agreement during transition. Once all consumers use the compiled catalog, the matrix is generated from concepts to remove dual authorship.
