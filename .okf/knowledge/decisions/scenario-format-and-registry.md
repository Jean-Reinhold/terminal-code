---
type: Architecture Decision
title: JSONC Scenarios and Compiled Registries
description: Author scenarios in strict JSONC and execute only versioned Rust registry operations.
tags: [adr, harness, jsonc, security]
status: draft
sources:
  - id: scenario
    resource: ../harness/scenario-dsl.md
    title: Scenario DSL
  - id: sandbox
    resource: ../harness/sandboxing.md
    title: Sandbox specification
  - id: current-jsonc
    resource: ../../../src/jsonc.ts
    title: Existing JSONC compatibility behavior
---

# Context

Humans and agents need a readable declarative scenario format. The runner needs strict schemas, typed values, deterministic compilation, safe paths, and stable evolution. Embedding shell or executable snippets would make generated scenarios an arbitrary-code interface and undermine sandbox guarantees.

# Decision

Author `*.scenario.jsonc` files validated against versioned JSON Schema. Compile them to canonical immutable run-plan nodes before side effects.

Scenarios may select only versioned Rust step, adapter, observation, normalizer, oracle, invariant, peer-state-machine, and target-manifest registry IDs. Arguments, paths, leases, targets, secrets, artifacts, and local peers are typed values. Unknown fields/IDs, absolute/traversing paths, arbitrary URLs, shell, eval, inline JavaScript/Rust, implicit interpolation, or inline executable normalizers are rejected.

Port and verify the existing source-preserving JSONC implementation before the harness depends on it. The scenario compiler preserves authored comments for review while execution uses canonical JSON.

# Consequences

Positive:

* Agent-authored scenarios are inspectable and policy-constrained.
* Schema/registry versions make execution and replay explicit.
* Existing JSONC expertise is reused.
* Sandboxing can validate all effects before target spawn.

Negative:

* New behavior requires a reviewed Rust registry extension rather than a quick script.
* Complex peer behavior needs compiled deterministic state machines.
* Schema migration must be maintained deliberately.

# Rejected Alternatives

* Shell/Bash scenario steps: unsafe, platform-dependent, and difficult to reason about.
* YAML: readable but introduces parser/dependency and implicit-typing concerns without existing project leverage.
* Rust tests only: safe but harder for agents/humans to generate, inspect, schedule, and link to contracts as data.
* Free-form Markdown execution: insufficiently machine-constrained.

# Acceptance

Stabilize after malicious scenario/path/URL/interpolation corpora fail before side effects and a complete C01/C02 vertical slice proves schema, compile, sandbox, observation, oracle, evidence, and replay.
