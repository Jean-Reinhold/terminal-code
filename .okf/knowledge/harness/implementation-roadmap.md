---
type: Implementation Plan
title: Agentic Harness Implementation Roadmap
description: H0-H8 deliverables, gates, parallel lanes, and integration with the Rust rewrite.
tags: [harness, roadmap, rust, migration]
status: draft
sources:
  - id: rewrite
    resource: ../plans/rust-rewrite.md
    title: Rust rewrite M0-M8 plan
  - id: architecture
    resource: architecture.md
    title: Harness architecture
  - id: ci
    resource: ci-and-platform-matrix.md
    title: CI and platform gates
---

# Ordering Rule

The harness is not a post-rewrite test project. H0-H3 precede load-bearing Rust migration so legacy behavior is captured before it moves. Agent orchestration begins only after deterministic schemas, execution, evidence, and oracle boundaries exist.

# H0 — Contract Decomposition and Coverage Graph

Deliver:

* convert C01-C22 from the matrix into individual OKF compatibility concepts;
* define harness frontmatter schema, risk/owner/surface vocabulary, staleness policy;
* create source/symbol/scenario/decision links;
* implement `catalog check` and canonical catalog emission;
* identify every unmapped production path and current test;
* assign current 119 tests to contracts or classify non-contractual/harness-only.

Acceptance:

* exactly one concept per C01-C22, no duplicate IDs;
* all concepts have owner/risk/surface/source evidence;
* every current test maps to at least one contract or explicit harness invariant;
* changed high-risk source without mapping fails a fixture check;
* OKF validation and catalog output are deterministic.

Integrates with rewrite M0.

# H1 — Scenario, Plan, and Evidence Core

Deliver:

* `tode-harness` crate with scenario JSONC parser/compiler and JSON Schemas;
* registry for steps/observations/normalizers/oracles;
* run-plan compiler, run/scenario/step state machines, append-only events;
* content-addressed object store, manifests, sealing, replay skeleton;
* policy engine, typed failure taxonomy, stable exit codes;
* self-tests for malformed schema, unknown registry IDs, artifact corruption, replay.

Acceptance:

* examples compile to canonical JSON identically across supported hosts;
* forbidden shell/path/URL/interpolation cases fail before side effects;
* an intentional observation/assertion mismatch produces complete replayable evidence;
* corrupt/missing artifact prevents pass;
* no model SDK/provider dependency enters `tode-harness`.

Integrates with rewrite M1.

# H2 — S0/S1 Sandbox and Core Adapters

Deliver:

* worker/sandbox roots, path containment, HOME/XDG/install environment, fixture clones;
* process groups, resource budgets, port/socket broker, loopback peer registry, teardown/leak scan;
* CLI/process, filesystem, Unix socket, HTTP/WebSocket, runtime fake, PTY/OSC adapters;
* exact/typed/differential/invariant oracles and initial approved normalizers;
* containment/fault self-tests.

Acceptance:

* deliberate user-home, symlink, archive, port-race, process-leak, and secret-canary attacks are caught;
* each adapter detects a deliberate relevant mutation;
* cancel/crash recovery removes owned processes/sockets without touching foreign resources;
* replay reproduces verdicts from sealed observations.

Integrates with rewrite M1-M2.

# H3 — Legacy Characterization and Differential Parity

Deliver:

* target manifests for legacy and Rust binaries/assets;
* scenarios wrapping existing target/theme/inject/live-sync/shortcut/import tests;
* missing CLI, process lifecycle, release worker, upgrade/uninstall, browser contracts frozen from legacy;
* normalizers only for allocated sandbox/port/PID/clock data;
* contract verdict and HTML/JUnit/SARIF reports.

Acceptance:

* every C01-C22 has at least one executable scenario or named S3 manual/hardware scenario;
* the harness detects seeded parity faults on every surface;
* legacy repeated runs are deterministic or documented inconclusive with a remediation owner;
* no baseline is accepted without exact review metadata;
* current suite remains runnable until its contract coverage is proven migrated.

Integrates with rewrite M0-M3 and gates implementation slices.

# H4 — Provider-Neutral Agent Plane

Deliver:

* `tode-harness-agent` crate with provider interface and exact model provenance;
* role prompt templates/output schemas for cartographer, planner, author, adversary, visual auditor, triager, skeptic, curator;
* task envelope, redaction, content/provenance cache, budgets, retry classification;
* proposal admission and scenario quality/mutation gate;
* agent DAG and outage/human-fallback policy;
* initial provider routes, including DeepSeek V4 Flash only when the runtime can attest actual invocation.

Acceptance:

* provider outage cannot suppress deterministic scenarios or fabricate output;
* requested-versus-reported model mismatch is visible and fails named-model policy;
* prompt-injection/fake-citation/secret-output fixtures are rejected;
* agent-authored scenario must compile, run deterministically, and kill a reviewed mutation before admission;
* cached outputs invalidate on any envelope/input/policy/model change.

Integrates after M1 and supports all later milestones.

# H5 — Browser, Terminal, Release, and Hardened Surfaces

Deliver:

* Rust browser driver spike and selected adapter with screenshot/DOM/a11y/network/console evidence;
* Ghostty/Kitty real adapters and S3 hardware worker protocol;
* hardened S2 VM/container worker for destructive/network/security scenarios;
* release archive/R2/worker/install/upgrade/rollback adapters;
* visual multi-signal oracle and reviewed thresholds;
* static self-contained report viewer.

Acceptance:

* embedded/public page scenarios exercise interaction, accessibility, responsive visuals, and failures;
* real terminal closed-loop/config reload/reset scenarios pass on dedicated hosts;
* staged release routes/ranges/HEAD/cache and transactional failures are reproducible;
* unsupported hard isolation on local macOS reports S1 and schedules S2 rather than overclaiming.

Integrates with rewrite M4-M7.

# H6 — CI Tiers, History, Fuzz, Mutation, and Chaos

Deliver:

* T0-T5 workflows with deterministic change impact and capability scheduling;
* native macOS/Linux matrix, hardware queue, nightly/release freshness policy;
* historical duration/co-failure/flakiness store keyed by scenario/platform/version;
* fuzz/property corpora, mutation definitions, chaos checkpoints;
* flake quarantine governance, retention, cost budgets, required agent stages.

Acceptance:

* PR workflow selects expected scenarios for a reviewed change corpus;
* cross-compilation is never reported as runtime pass;
* critical contracts cannot be silently quarantined/skipped;
* retained fuzz seeds and mutation survivors create reproducible scenarios/findings;
* superseded/cancelled runs clean resources and retain classified evidence.

Integrates with rewrite M2-M7.

# H7 — Release Certification and Supply-Chain Attestation

Deliver:

* build-once artifact identity, immutable staged namespace, full T5 certification;
* release certificate schema, evidence-root signing, independent verifier;
* latest-pointer policy requiring valid certificate and exact artifact digests;
* rollback certificate/path, post-publication smoke, incident evidence flow;
* OKF verification/update proposals from accepted runs.

Acceptance:

* publication cannot move latest without a valid nonexpired certificate;
* certificate verification works from a clean environment without model access;
* every supported artifact installs, launches offline, upgrades, rolls back, and uninstalls;
* transaction failure at each checkpoint leaves previous latest/install usable;
* OKF curator proposal fails on stale concept hashes and never auto-promotes status.

Integrates with rewrite M7.

# H8 — Clean-Cutover Proof and Steady State

Deliver:

* final full parity run and release certificate;
* reachability/catalog scan proving no legacy production path remains;
* migration of valuable current tests to harness/Rust ownership and deletion of duplicate Node suite;
* deletion of compatibility target only after rollback artifact/evidence retention is established;
* ongoing contract freshness, upstream-pin, nightly/hardware, security, and agent-review schedules;
* harness maintenance/incident/runbook ownership.

Acceptance:

* all M8 and T5 gates pass with no unexplained differential/inconclusive critical contract;
* repository/build/release manifests contain no reachable legacy application implementation;
* deterministic suites run during total provider outage;
* release evidence is replayable for the audit/rollback horizon;
* human owners accept stable contracts, host-boundary decisions, and operational runbooks.

Integrates with rewrite M8.

# Crosswalk

| Harness | Rewrite | Dependency |
|---|---|---|
| H0 | M0 | contract freeze |
| H1-H2 | M1 | workspace and compatibility infrastructure |
| H3 | M0-M3 | legacy characterization before/alongside pure/state ports |
| H4 | M1+ | agentic expansion after deterministic boundary |
| H5 | M4-M7 | runtime/protocol/shortcut/web/release surfaces |
| H6 | M2-M7 | continuous risk and platform gates |
| H7 | M7 | staged release certification |
| H8 | M8 | clean-cutover proof |

# Parallel Lanes

After H1 schemas are stable:

* sandbox/adapters (H2);
* contract decomposition/scenario characterization (H0/H3);
* artifact/report/replay implementation (H1/H3);
* CI worker/capability foundations (H6);
* agent prompt/output schema design, without provider execution until H4 trust controls exist.

After H2/H3:

* browser/terminal hardware;
* release worker/installer scenarios;
* agent providers/roles;
* fuzz/mutation/chaos campaigns.

Cross-lane interface changes update schema/version/fixtures/OKF concepts and all consumers together.

# Review Units

A harness change is complete only when it includes:

* schema/contract change if applicable;
* safe implementation;
* self-test proving relevant mismatch/attack detection;
* example or real scenario;
* observation/oracle/artifact evidence;
* documentation/OKF update;
* no placeholder adapter, no disabled gate, and no unbounded follow-up.

# First Implementation Slice

The first code slice should implement C01/C02 target/CLI pure characterization end to end:

1. H0 concepts for C01/C02.
2. Scenario schema/compiler subset for `process.exec` and `process.result`.
3. S1 sandbox HOME/workspace/process group.
4. exact stdout/stderr/exit and filesystem observations.
5. differential oracle and content-addressed evidence/replay.
6. scenarios for help/version/target/goto/invalid arguments.
7. deliberate mutation proving failure detection.

This validates the entire trust/evidence vertical before expanding the vocabulary.
