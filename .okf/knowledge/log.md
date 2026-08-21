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
