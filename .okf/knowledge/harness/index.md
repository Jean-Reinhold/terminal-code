# Agentic Validation Harness

* [Principles](principles.md) - Authority hierarchy, invariants, goals, and non-goals.
* [Implementation status](implementation-status.md) - Implemented C01/C02 deterministic vertical slice, proof, and remaining boundaries.
* [Architecture](architecture.md) - Rust components, control/data planes, scheduler, and repository layout.
* [Contract catalog](contract-catalog.md) - OKF-backed contract registry, coverage graph, and impact selection.
* [Scenario DSL](scenario-dsl.md) - Versioned JSONC scenario format and safe executable vocabulary.
* [Run state machine](run-state-machine.md) - Lifecycle, retries, sharding, resumption, and verdict states.
* [Sandboxing](sandboxing.md) - Filesystem, process, port, network, terminal, clock, and secret isolation.
* [Surface adapters](surface-adapters.md) - CLI, filesystem, IPC, HTTP, process, PTY, browser, release, and hardware adapters.
* [Oracles and normalization](oracles-and-normalization.md) - Exact, semantic, differential, invariant, visual, and human-reviewed comparisons.
* [Agent orchestration](agent-orchestration.md) - Provider-neutral roles, task envelopes, DAGs, trust boundaries, and outage behavior.
* [Evidence and artifacts](evidence-and-artifacts.md) - Content-addressed observations, provenance, reports, replay, and retention.
* [CI and platform matrix](ci-and-platform-matrix.md) - PR, nightly, hardware, staged, and release-certification gates.
* [Security and adversarial validation](security-and-adversarial.md) - Threat model, fuzzing, mutation, chaos, prompt-injection, and tamper controls.
* [Implementation roadmap](implementation-roadmap.md) - H0-H8 deliverables and integration with rewrite milestones M0-M8.
