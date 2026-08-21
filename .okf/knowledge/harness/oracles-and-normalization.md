---
type: Oracle Specification
title: Deterministic Oracles and Normalization
description: Comparison hierarchy, approved nondeterminism removal, baseline policy, and inconclusive handling.
tags: [harness, oracle, normalization, differential, visual]
status: draft
sources:
  - id: parity
    resource: ../verification/parity-strategy.md
    title: Legacy-to-Rust parity strategy
  - id: adapters
    resource: surface-adapters.md
    title: Validation surface adapters
  - id: scenarios
    resource: scenario-dsl.md
    title: Scenario DSL
---

# Principle

An oracle answers one explicit contract question using sealed observations and a reviewed rule. It cannot infer a pass from missing data, model confidence, retry success, or visual plausibility.

# Oracle Hierarchy

Use the strongest applicable oracle:

1. **Exact** — raw bytes, exit status, JSON omission/order where contractual, filesystem bytes/modes, archive entries, request/response fields.
2. **Typed semantic** — parsed JSON/JSONC, process event state, HTTP semantics, keymap/config model, accessibility tree, manifest schema.
3. **Differential** — legacy and Rust receive independently cloned equivalent inputs; normalized observations must match.
4. **Invariant** — safety/idempotence/convergence/no-leak/atomicity/property rules independent of implementation.
5. **Metamorphic** — controlled input transformation must produce a defined output relation.
6. **Visual multi-signal** — screenshot metric plus DOM, accessibility, focus, console, and interaction assertions.
7. **Human-reviewed** — only where no deterministic rule can establish acceptability; result remains review-required until signed.

Agents can explain evidence at every level but cannot act as the oracle.

# Differential Authority

Legacy parity is evidence, not absolute product authority. Outcomes:

* Legacy and Rust match stable contract: pass.
* Rust differs and stable contract agrees with legacy: fail Rust.
* Rust differs and stable contract explicitly agrees with Rust: legacy discrepancy; pass only after reviewed contract scenario, never by differential equality alone.
* Contract is ambiguous or legacy appears buggy: `inconclusive/contract-conflict`, open a decision; do not encode either output as baseline automatically.
* Both implementations violate an invariant: fail contract/safety despite differential equality.

This prevents faithfully porting known accidental bugs without review while still blocking unapproved behavior cleanup.

# Normalizer Registry

Each normalizer is Rust code identified by `<domain>.<name>-vN` and declares:

* accepted observation schema versions;
* exact fields/byte ranges it may transform;
* output schema;
* rationale and linked contracts;
* risk tiers/suites where allowed;
* deterministic fixtures showing changed and unchanged data;
* information-loss classification.

Examples:

* `path.sandbox-root-v1`: replace the exact allocated sandbox prefix with `$SANDBOX`.
* `lease.port-v1`: map broker lease values to `$PORT:<name>`.
* `process.pid-v1`: map harness-owned recorded PIDs to stable symbolic IDs.
* `clock.invocation-time-v1`: normalize the recorded invocation timestamp where the contract excludes it.

Not allowed as broad defaults:

* sorting arbitrary arrays or lines;
* dropping stderr, unknown JSON fields, headers, paths, stack frames, or console errors;
* regex-removing all numbers/paths/timestamps;
* masking arbitrary screenshot regions;
* rewriting error wording;
* rounding timing beyond contract-defined tolerance.

High-information-loss normalizers require explicit per-scenario approval and surface a warning in reports.

# Oracle Trace

Every assertion emits:

```text
assertion_id
oracle_id/version
contract_ids
input observation digests
normalizer chain IDs/versions
normalized output digests
expected source/digest
comparison result
structured diff artifact
reason/classification
```

A replay loads only these referenced artifacts and verified runner code; it does not call agents or targets.

# Baselines

Baselines are appropriate for stable expected bytes/images/trees, not as a substitute for contract prose. A baseline record includes contract/scenario/observation IDs, source run, prior/new digest, tool versions, platform scope, reason, reviewer, approval time, and expiry/revalidation policy.

Workflow:

1. A run produces a candidate observation.
2. Harness creates a proposal and diff; repository remains unchanged.
3. Agent reviewers may explain/attack the proposal.
4. An authorized human approves the exact digest and contract rationale.
5. A deterministic command copies content to `harness/baselines`, updates metadata, and logs the change.
6. CI verifies baseline metadata and review policy.

Bulk “accept all” is forbidden for critical/high contracts.

# Visual Oracles

Visual validation combines:

* exact viewport/scale/locale/timezone/font/browser/theme fixture;
* screenshot dimensions and perceptual/pixel metrics;
* reviewed small anti-alias tolerance, not broad masking;
* DOM structure/text/attributes relevant to the contract;
* accessibility roles/names/states/focus order;
* interaction outcomes and URL/storage/network changes;
* console error and resource failure checks.

Thresholds are calibrated with known acceptable and deliberately broken examples. A score near the boundary is `inconclusive` and requires review. Layout regions can be masked only for explicitly non-contractual dynamic data with a linked policy.

Agent visual auditors receive before/after images and deterministic diffs. They identify likely semantic issues or missing assertions; they do not override metric/DOM/a11y failure.

# Timing and Performance

Functional timing contracts use explicit deadlines/ordering, controlled clocks, and event traces. Performance characteristics use distributions from repeated isolated runs, warm/cold labels, machine class, confidence intervals, and approved regression budgets. One fast retry cannot hide a slow failure.

The rewrite initially preserves observable timeouts and startup stage ordering. New performance targets require separate decisions.

# Inconclusive

Return inconclusive when:

* required observation could not be captured;
* platform/capability is absent;
* visual result lies in a review band;
* stable contract conflicts with both implementations;
* normalizer/baseline provenance is invalid;
* nondeterminism produces mixed product outcomes;
* required human/agent review is unavailable.

Inconclusive blocks required CI/release gates. It is never automatically downgraded to warning.

# Metamorphic and Invariant Examples

* Applying settings/shortcut decisions twice changes no byte.
* Complete shortcut decisions followed by rescan yield no unresolved conflicts.
* Moving sandbox root changes only normalized path fields.
* Reordering unrelated JSONC keys/comments does not change managed setting semantics and preserves unrelated bytes.
* Truncating a release artifact always prevents unpack/swap.
* Wrong size or SHA-256 never produces an accepted runtime/install.
* Closing one live theme socket does not prevent updates to remaining sockets and removes the dead socket.
* Adding an unknown safe manifest field does not break tolerant reading but is preserved where round-trip policy requires it.

# Oracle Mutation Tests

Each oracle/normalizer is itself validated by mutations that change relevant and irrelevant fields. Relevant mutations must fail; irrelevant mutations must remain equal only when contract policy says so. This guards normalizers that accidentally erase behavior.
