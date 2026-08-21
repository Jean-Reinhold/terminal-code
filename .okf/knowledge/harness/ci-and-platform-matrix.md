---
type: CI Strategy
title: Harness CI and Platform Matrix
description: Ordered PR, nightly, hardware, staged, and release-certification gates with agent outage and flake policy.
tags: [harness, ci, platform, release]
status: draft
sources:
  - id: current-ci
    resource: ../../../.github/workflows/release.yml
    title: Current release workflow
  - id: parity
    resource: ../verification/parity-strategy.md
    title: Current parity strategy
  - id: operations
    resource: ../operations/release-and-supply-chain.md
    title: Release and supply-chain plan
---

# Baseline Gap

The current repository has 119 Node tests across eight files. The release workflow runs the same suite on Ubuntu while packaging `darwin-arm64`, `linux-x64`, and `linux-arm64`; it does not execute on macOS, exercise native target binaries, validate the public/embedded browser surfaces, test the release-worker HTTP contract, run terminal hardware, or stage release certification before publication.

# Workflow Tiers

## T0 — Static Catalog and Harness Integrity

Every pull request:

* OKF v0.2 structure, sources, links, contract schema, ownership, staleness;
* scenario JSONC parsing/schema/registry/path/policy validation;
* reciprocal contract/scenario/fixture/baseline references;
* Rust format, focused lint, dependency/license/advisory policy;
* generated catalog/schema/example drift;
* harness self-tests, containment canaries, oracle mutation tests;
* changed production symbols mapped to contracts.

No agents required. Target: minutes.

## T1 — Impacted Deterministic PR Run

After T0:

* build affected native/WASM crates;
* deterministic impact graph plus critical sentinels;
* S0 pure differential/property scenarios;
* S1 CLI/filesystem/IPC/HTTP/process scenarios on Linux;
* fixture/adapter/oracle tests for changed harness code;
* legacy-vs-Rust parity for implemented migration slices;
* JUnit/SARIF/evidence summary upload.

Agents can add scenarios before plan sealing. Agent outage does not remove deterministic selections.

## T2 — Platform PR Run

Required when impact includes platform/runtime/profile/shortcuts/distribution or policy marks risk high/critical:

* native macOS and Linux runners;
* architectures available as real hosted/self-hosted runners, otherwise compile-only clearly separated;
* Ghostty/Kitty fixture modes;
* browser routes/interactions at fixed viewports;
* install/upgrade/uninstall in hardened sandboxes;
* required agent skeptic/visual review according to risk.

## T3 — Full Nightly

Scheduled from the protected main commit:

* all C01-C22 scenarios on macOS/Linux;
* complete legacy-vs-Rust matrix while migration is active;
* property/fuzz campaigns with retained seeds;
* mutation campaigns over changed/high-risk crates and oracle/normalizer code;
* network/process/filesystem chaos scenarios;
* repeated deterministic runs for flake detection;
* dependency/upstream pin freshness and generated keymap checks;
* full browser responsive/accessibility/visual suite;
* agent cartographer/adversary/skeptic sweeps with bounded budgets.

Nightly findings cannot be ignored merely because PR gates passed.

## T4 — Hardware and Integration

Scheduled and pre-release on dedicated ephemeral machines:

* actual Ghostty and Kitty versions with isolated configs and reload signals;
* Kitty graphics protocol plus terminal-browser rendering;
* real code-server pinned version, bridge activation, IPC reuse/wait, live themes;
* browser/display/font behavior on macOS/Linux;
* cold/offline first launch and real process cleanup;
* S3 reset/containment attestation.

Hardware jobs are serialized per device/session and retain failure traces/screenshots.

## T5 — Staged Release Certification

For a release candidate built once:

1. verify artifact provenance, archive safety/layout, size/SHA-256, manifests;
2. upload immutable objects into a non-production namespace;
3. deploy Rust worker/site to staging;
4. execute route/range/HEAD/cache/install contract suite;
5. clean install every supported target;
6. cold/offline launch, window reuse, representative shortcut/theme/import flow;
7. upgrade from current stable and previous dev; interrupt each transaction checkpoint;
8. rollback through the same verified installer path;
9. uninstall and verify retained/removed paths;
10. run required security/adversarial and agent skeptic reviews;
11. seal and sign release certificate.

Only the signed certificate can authorize latest-pointer movement.

# Gate Ordering

```text
T0 catalog/harness
  -> T1 impacted deterministic
  -> conditional T2 platforms
  -> merge
  -> T3 nightly + T4 hardware freshness
  -> release build once
  -> T5 staged certification
  -> immutable publication
  -> latest pointer
  -> post-publication smoke
```

Worker deployment must not precede its route contract tests and staged certification, unlike the current independent deployment job.

# Platform Matrix

| Surface | Linux x64 | Linux arm64 | macOS arm64 | macOS x64 | Hardware requirement |
|---|---:|---:|---:|---:|---|
| pure/schema/oracle | required | required or native periodic | required | required or native periodic | no |
| CLI/state/IPC/process | required native | required native before release | required native | required native if published | no |
| injector/WebSocket | required | required | required | required if published | no |
| Ghostty/Kitty fixture | required | required | required | required if published | no |
| real Ghostty/Kitty | periodic/release | release where supported | periodic/release | if published | yes |
| browser UI | required | periodic | required | if published | browser/display |
| terminal-browser graphics | release | release | release | if published | terminal/display |
| install/upgrade/uninstall | required | required | required | required if published | hardened VM/host |
| worker WASM | one deterministic build + staged runtime tests | artifact-independent | artifact-independent | artifact-independent | staged Cloudflare-compatible runtime |

Cross-compilation is compile evidence only and is never labeled runtime validation.

# Change Impact Policy

Paths map to minimum tiers. Examples:

* `tode-protocol`, scenario schemas, normalizers, oracles: T0 + all impacted consumers; high-risk changes trigger T3 subset.
* runtime/IPC/profile/shortcuts: T1 + T2 native platforms.
* browser/UI/CSS/assets/fonts: T1 + browser T2; visual agent review where required.
* installer/upgrade/release worker/xtask/workflow: T0-T2 plus mandatory staged T5 rehearsal.
* sandbox/security/policy: full harness self-tests, containment suite, security review; no risk-based narrowing.

Agents may raise tiers; only reviewed policy changes lower them.

# Agent Availability

| Stage | Outage behavior |
|---|---|
| optional PR discovery/triage | deterministic run proceeds; report marks skipped stage |
| required critical-risk cartographer/skeptic | gate inconclusive; retry or authorized human fallback |
| visual audit with deterministic pass outside review band | policy decides optional/required |
| visual result in review band | required review; outage blocks |
| release adversarial/skeptic review | blocks certificate unless explicit signed exception policy applies |

Substituting another model/provider creates a new recorded task and must satisfy any diversity/named-model policy.

# Flake Policy

A mixed product result across identical fresh attempts is a failure/inconclusive, not solved by retry. Quarantine requires:

* issue, owner, contract IDs, evidence, root-cause hypothesis;
* expiry date and maximum duration;
* replacement coverage so no critical contract disappears;
* separate visible CI job that continues running the quarantined scenario;
* prohibition on release certification quarantine for critical user-state, security, or publication scenarios.

Infrastructure flake and product nondeterminism are separate classifications.

# Cost Controls

* deterministic change impact and sharding by historical duration;
* content/agent proposal caches keyed by full provenance;
* progressive artifact upload and retention by verdict;
* bounded agent tasks and generated scenario count;
* sampled long fuzz campaigns with retained corpus, not sampled required contracts;
* reusable clean build artifacts identified by digest;
* cancellation of superseded PR runs only before irreplaceable staged/release transactions.

Cost policy never drops required critical scenarios silently.

# Release Certificate

The canonical certificate includes source/build/artifact/manifest/evidence-root digests, target/platform matrix, required contract verdicts, nightly/hardware freshness, agent tasks and exact model provenance, security findings/exceptions, baseline/normalizer versions, rollback artifact, signer identity, and expiry.

Latest publication verifies the certificate against policy and exact artifacts. A successful GitHub workflow status without a matching certificate is insufficient.
