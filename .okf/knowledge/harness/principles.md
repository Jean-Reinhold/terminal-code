---
type: Harness Principle
title: Agentic Validation Harness Principles
description: Authority hierarchy and invariants for agent-assisted but deterministic validation.
tags: [harness, agents, verification, principles]
status: draft
sources:
  - id: compatibility
    resource: ../contracts/compatibility.md
    title: Behavioral compatibility matrix
  - id: parity
    resource: ../verification/parity-strategy.md
    title: Legacy-to-Rust parity strategy
  - id: constraints
    resource: ../project/rewrite-constraints.md
    title: Rust rewrite constraints
---

# Mission

Build one harness that can prove the Rust implementation preserves terminal-code behavior, continuously discover missing coverage, and leave a replayable evidence trail. The harness must remain useful when every model provider is unavailable.

# Definition of Agentic

Agents perform work that benefits from semantic exploration:

* map changed code to contracts and risk;
* identify untested claims and ambiguous requirements;
* propose scenario specifications and fixtures;
* generate boundary, metamorphic, fault, and adversarial variants;
* review screenshots, accessibility trees, traces, and diffs;
* cluster failures and propose likely causes;
* challenge a passing run for missing evidence;
* draft OKF updates backed by run artifacts.

Deterministic Rust code performs work that establishes truth:

* validate contract and scenario schemas;
* compile safe scenario steps;
* provision isolated sandboxes;
* execute programs and adapters;
* capture observations;
* apply approved normalizers and oracle rules;
* calculate pass, fail, inconclusive, or infrastructure-error verdicts;
* hash, store, and replay evidence;
* enforce CI and release policies.

An agent opinion is never a passing verdict.

# Authority Hierarchy

From highest to lowest:

1. Approved product contract and explicit human-reviewed decision.
2. Versioned scenario, fixture, normalizer, and baseline committed to the repository.
3. Deterministic observation and oracle output from an identified build.
4. Agent proposal or interpretation with complete provenance.
5. Unattributed prose or model confidence.

Lower authority cannot overwrite higher authority. Conflicts become visible `inconclusive` or review-required outcomes.

# Non-Negotiable Invariants

1. **Hermetic by default**: each scenario receives a unique HOME, XDG roots, working tree, ports, sockets, processes, and local services.
2. **No arbitrary scenario code**: generated scenarios select versioned step/adapter/normalizer IDs; they cannot embed shell, scripts, eval, or network destinations.
3. **Evidence before verdict**: every assertion points to stored observations and the exact rule that compared them.
4. **Reproducible identity**: run ID derives from commit/build, scenario digest, fixture digests, policy digest, platform capability, and target pair.
5. **Provider-neutral**: model/provider adapters are outside the deterministic harness core; runs record the exact provider/model when agents contribute.
6. **Outage-safe**: known deterministic suites still execute if all agents fail. Required agent review becomes a visible missing gate, never a fabricated success.
7. **Fail closed on trust**: malformed schemas, missing evidence, unknown normalizers, baseline drift, sandbox containment failure, or unverifiable model provenance cannot pass.
8. **No real user mutation**: the harness refuses unsafe path resolution before spawning any target.
9. **No silent retry**: retries are limited to classified infrastructure failures and every attempt remains in evidence.
10. **No automatic baseline approval**: agents may propose a new baseline; an authorized reviewer must approve its exact digest and justification.
11. **One source of contract truth**: compatibility contracts live as OKF concepts; generated catalogs are derived artifacts.
12. **Clean cutover proof**: release certification proves no reachable legacy production path remains.

# Goals

* Select all scenarios affected by a code or contract change, plus risk-based regression sentinels.
* Run independent scenarios and agent tasks in parallel without shared mutable state.
* Differentially compare legacy and Rust until clean cutover, then preserve the same scenarios as Rust regression contracts.
* Validate CLI, filesystem, IPC, OSC, HTTP/WebSocket, process, terminal, browser, release-worker, installation, upgrade, rollback, and uninstall surfaces.
* Produce human-readable reports and machine-readable JUnit, SARIF, JSON, and OKF update proposals.
* Make any verdict replayable from stored inputs without contacting a model.

# Non-Goals

* Replacing deterministic tests with LLM judgement.
* Allowing agents to execute unrestricted repository code.
* Treating visual similarity alone as semantic UI correctness.
* Hiding flakes through retries or wide tolerances.
* Running every platform/hardware scenario on every local edit.
* Adding unrelated product features during the rewrite.

# Success Measures

* Every C01-C22 contract has at least one executable scenario and named owner.
* Every changed production symbol maps to a contract or produces a catalog-gap failure.
* Every release verdict can be replayed from retained artifacts.
* A deliberately injected mismatch is detected on every supported observation surface.
* Mutation campaigns demonstrate that contract scenarios kill representative plausible bugs.
* Agent outage leaves deterministic gates operational and reports exactly which discovery/review stages were skipped.
