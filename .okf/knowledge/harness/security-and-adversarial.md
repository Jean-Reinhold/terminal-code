---
type: Security Plan
title: Harness Security and Adversarial Validation
description: Threat model and deterministic fuzz, property, mutation, chaos, and hostile-agent campaigns.
tags: [harness, security, fuzzing, mutation, chaos]
status: draft
sources:
  - id: sandbox
    resource: sandboxing.md
    title: Sandbox specification
  - id: agents
    resource: agent-orchestration.md
    title: Agent orchestration
  - id: operations
    resource: ../operations/release-and-supply-chain.md
    title: Release and supply-chain plan
---

# Protected Assets

* real user files, terminal configs, processes, credentials, and network services;
* approved contracts, scenarios, normalizers, baselines, and policies;
* build/release artifacts, manifests, latest pointers, signing identities, and R2 objects;
* run evidence, provenance, verdicts, and release certificates;
* agent task boundaries, redacted inputs, and model/provider identity;
* CI workers, hardware terminals/displays, and artifact storage.

# Trust Boundaries

Untrusted:

* repository/source comments and strings;
* legacy/Rust target output;
* fixtures and archives under test;
* HTTP/web content and screenshots/OCR;
* agent prompts generated from repository data and every agent response;
* external model/provider metadata unless independently recorded;
* downloaded upstream/runtime/release artifacts until verified.

Trusted only after validation:

* compiled scenario/run plan;
* target/worker capability manifest;
* registered Rust adapters/normalizers/oracles;
* sealed content-addressed evidence;
* authorized human/process decisions and release signatures.

# Threats and Controls

| Threat | Deterministic control | Required campaign |
|---|---|---|
| Scenario arbitrary execution | strict JSON Schema, typed registry, no shell/eval/raw URLs | malicious scenario corpus |
| Path/symlink/hardlink escape | dir-handle-relative no-follow operations, containment preflight | filesystem race/escape suite |
| Archive traversal/bomb/device | streaming validation, count/size/type/link budgets | fuzzed hostile archives |
| Process/socket leak | process groups, lease broker, teardown scan | crash/timeout/cancel chaos |
| Real HOME/config mutation | mandatory absolute sandbox env, destructive path proof, canaries | uninstall/upgrade hostile-env suite |
| Port hijack/race | held listener transfer/proxy, authenticated lease metadata | concurrent foreign bind attack |
| Secret/PII exfiltration | opaque handles, typed redaction, scanners, minimal agent views | seeded canary campaign |
| Prompt injection | source as delimited data, no tools/write, schema output, fixed policy | hostile comments/log/web content |
| Agent fabricated evidence | artifact references validated against declared inputs | fake path/digest/citation responses |
| Agent weakens gate/baseline | proposal-only permissions, human digest approval, policy signatures | malicious proposal campaign |
| Model substitution/provenance lie | requested/reported provider identity record, policy check | wrapper/model mismatch fixture |
| Baseline/normalizer tamper | reviewed metadata, digest graph, oracle mutation tests | information-loss mutations |
| Artifact poisoning | content addressing, immutable keys, read-back verification | conflicting object/write race |
| CI/release credential abuse | least privilege, staged namespace, workload identity, signed certificate | publication failure/rollback drill |
| Flake laundering | product/infrastructure classification, retained attempts, no product retries | nondeterminism injection |

# Fuzzing

Targets:

* JSONC parser/editor and scenario/catalog parsers;
* goto/target/chord/terminal-trigger parsers;
* OSC/ANSI reply parser with chunking and malformed terminators;
* Unix JSONL framing and reply parser;
* HTTP header/body/injection/rewrite logic and WebSocket upgrade handling;
* release manifests, archive readers, installer receipts;
* Ghostty/Kitty keymap/action/config parsers;
* normalizers and observation decoders.

Use coverage-guided fuzzing for byte parsers and structured generators for schemas. Store minimized failing input, seed, harness/target digests, classification, and reproducer scenario. Fuzzers run in S0/S1/S2 appropriate to impact; archive/destructive targets require hardened isolation.

A crash, hang, budget escape, nondeterministic decode, or containment violation is a finding even when legacy behaves the same.

# Property Testing

Representative properties:

* parse/serialize/parse semantic stability where serialization is defined;
* source-preserving edits leave unrelated JSONC bytes/comments intact;
* repeated settings/shortcut/config apply is byte-idempotent;
* chord/trigger conversions invert for supported names;
* palette channels remain bounded and required contrast invariant holds;
* path resolution never escapes sandbox for any component sequence;
* normalizers are deterministic/idempotent and preserve relevant mutations;
* manifest/download verification accepts only exact declared size/hash;
* shortcut resolution converges within a bounded state space;
* artifact root digest changes when any reachable byte changes.

# Mutation Testing

Run mutations against product Rust crates and harness trust code. Categories:

* delete validation branch;
* invert condition/precedence;
* change timeout/default/exit/status/message;
* omit/reorder protocol field or newline;
* skip atomic rename/fsync/verification;
* weaken path/hash/size check;
* ignore process child/socket;
* broaden normalizer deletion/tolerance;
* change visual threshold;
* skip required platform/agent gate;
* publish latest before immutable objects/certificate.

Critical scenario sets must kill reviewed representative mutations. Surviving mutations create coverage work, not automatic code rejection when semantically equivalent; an agent can triage but deterministic reproduction decides.

# Chaos and Fault Injection

Reviewed checkpoints inject:

* download truncation, wrong size/hash, disconnect, slow body, redirect, stale cache;
* upstream/code-server slow readiness, malformed response, crash after listening;
* injector WebSocket half-close/reset;
* disk full, permission denied, interrupted write/rename, read-only directory;
* stale PID/state/port/socket, PID reuse using harness-owned processes;
* process signal during each install/upgrade/uninstall phase;
* R2 upload/read-back/latest-pointer failure and concurrent publisher;
* browser navigation/resource failure and dead live-theme socket;
* terminal no/partial/delayed/interleaved OSC replies;
* coordinator/worker crash at every run-state transition;
* model authentication/rate-limit/timeout/malformed/provenance mismatch.

Fault IDs and activation events are part of evidence.

# Adversarial Agent Campaigns

Agents receive contracts and bounded evidence to propose attacks, not execution privileges. Campaign roles:

* implementation adversary: plausible parity-breaking change;
* scenario adversary: bypass/under-specification in DSL or policy;
* oracle adversary: relevant difference erased by normalizer/tolerance;
* evidence adversary: fabricated citation/digest/partial observation;
* prompt adversary: instructions embedded in comments/logs/web content;
* release adversary: transaction ordering/rollback/supply-chain exploit;
* skeptic: why a passing certificate is insufficient.

Every proposal passes schema/policy review and runs in deterministic isolation. Agent success means it found a reproducible issue; prose alone is a hypothesis.

# Security Severity

* Critical: real-user mutation, sandbox escape, secret leak, unverified artifact accepted, unauthorized latest publication, forged/tampered certificate.
* High: protocol/auth boundary bypass, process persistence, baseline/oracle weakening that permits false pass, prompt injection changing plan/gate.
* Medium: bounded denial of service, artifact/log exhaustion within worker, missing noncritical evidence.
* Low: diagnostic/report issue without verdict or containment impact.

Critical/high findings block relevant PR/release gates until fixed or an explicitly governed exception exists; clean-cutover certification permits no critical exceptions.

# Security Release Gates

1. Containment canary and malicious scenario/archive suites pass on each supported OS class.
2. No secret canary appears in sealed/uploaded/agent-visible artifacts.
3. Critical parser/sandbox/oracle code passes fuzz/property and representative mutation thresholds.
4. Staged release transaction survives every required failure checkpoint and rolls back.
5. Agent prompt-injection/provenance/fabrication fixtures are rejected visibly.
6. Dependency/advisory/license policy passes for native and WASM crates.
7. Release certificate and artifacts verify from an independent clean verifier.
8. No unresolved critical/high finding affects the release path.

# Incident Evidence

Security failures retain restricted raw evidence, public-safe summary, affected contracts/runs/artifacts, containment status, exact target/harness versions, minimized reproducer where safe, and revocation/rollback actions. Agents receive only redacted minimum context.
