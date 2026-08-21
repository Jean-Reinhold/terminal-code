---
type: Evidence Model
title: Content-Addressed Evidence and Reports
description: Immutable run manifests, observations, provenance, replay, reporting, redaction, and retention.
tags: [harness, evidence, artifacts, provenance]
status: draft
sources:
  - id: execution
    resource: run-state-machine.md
    title: Run state machine
  - id: agents
    resource: agent-orchestration.md
    title: Agent orchestration
  - id: oracles
    resource: oracles-and-normalization.md
    title: Oracle model
---

# Evidence Root

Default local layout:

```text
<harness-artifact-root>/runs/<run-id>/
  run.json
  plan.json
  events.jsonl
  catalog.json
  environment.json
  targets.json
  scenarios/<scenario-id>/
    scenario.json
    attempts/<n>/
      events.jsonl
      observations.json
      assertions.json
      teardown.json
  agents/<task-id>/
    envelope.json
    response.json
    validation.json
  verdicts/
    scenarios.json
    contracts.json
    run.json
  reports/
    summary.txt
    report.html
    junit.xml
    findings.sarif
    okf-proposals.json
  refs.json
<artifact-root>/objects/sha256/<prefix>/<digest>
```

Metadata files are canonical JSON with stable key ordering and schemas. Large/raw content lives once in the object store and is referenced by digest, media type, byte length, and redaction status.

# Run Manifest

`run.json` records:

* run/run-key, source commit, dirty patch digest, repository identity;
* legacy/Rust build and executable digests;
* Rust toolchain, compiler/build flags, lockfile, upstream runtime versions;
* catalog/scenario/policy/fixture/baseline/normalizer/oracle digests;
* host/worker OS, architecture, kernel, machine class, capabilities;
* coordinator/worker/harness versions;
* selected scenarios and reasons;
* agent requirements and actual task IDs/models/statuses;
* start/end and final classified verdict;
* artifact index/root digest.

Environment records include allowlisted key names and redacted/value digests where necessary, never secret values.

# Observation Record

```json
{
  "observation_id": "ipc.window-reuse.wait/right/wire",
  "kind": "unix-socket-transcript-v1",
  "producer": "unix-socket-adapter-v1",
  "scenario": "ipc.window-reuse.wait",
  "target": "right",
  "step": "window",
  "content": { "digest": "sha256:...", "bytes": 123, "media_type": "application/json" },
  "captured_at_event": 42,
  "redaction": { "policy": "evidence-redaction-v1", "changed": false },
  "schema": "observation/unix-socket-transcript-v1"
}
```

Raw and derived observations are separate records with lineage. A normalized observation references raw input plus normalizer trace; it never overwrites raw evidence.

# Artifact Sealing

Workers stream temporary captures, enforce budgets/redaction, calculate SHA-256, write objects atomically, then emit an observation record. The coordinator accepts only digest-valid objects and immutable records. Final run sealing calculates an index/root digest over all references.

Remote storage uses write-once object keys and separate mutable retention/index metadata. Release certificates reference the sealed root digest.

# Replay

`tode-harness replay <run-id>` verifies schemas/digests, loads sealed observations and exact normalizer/oracle versions, and recalculates assertion/verdict records without running targets or agents. Replay reports code/version unavailability as inconclusive rather than substituting current logic.

A deeper `rerun` command executes the sealed scenario/fixture/policy against available matching target builds in a new run ID; it never mutates the original.

# Reports

* `summary.txt`: concise terminal result, gate failures, replay command, artifact location.
* `report.html`: static, self-contained navigation through contracts/scenarios/evidence/diffs/agent provenance; no external script/data upload.
* `junit.xml`: scenario nodes and classified failures for CI.
* `findings.sarif`: source/contract-linked policy, security, and mismatch findings.
* `okf-proposals.json`: machine proposals referencing sealed evidence; never applied automatically.
* release certificate: signed canonical JSON containing required gates and evidence root.

Reports distinguish facts, deterministic inference, agent hypothesis, and human decision visually and structurally.

# Redaction

Redaction happens before artifact sealing and agent access. Policies are typed per observation kind; generic regex-only redaction is insufficient for JSON, HTTP, environment, screenshots, and filesystem captures.

Controls:

* secret handles never expose values to scenario plans;
* structured fields are removed/replaced before serialization;
* byte/log scanners use known canaries and credential patterns;
* screenshot regions require deterministic element-bound redaction, with an explicit record;
* redacted and unredacted digests are not both uploaded by default;
* a redaction that affects asserted content makes the scenario invalid/inconclusive unless contract policy defines it.

# Retention

| Class | Retention |
|---|---|
| PR pass | summary, manifests, verdicts; short-lived heavy objects |
| PR failure/inconclusive | complete evidence long enough for diagnosis/replay |
| nightly fuzz/mutation | failing/minimized inputs and campaign summaries; discard redundant passes |
| release certification | complete sealed evidence for supported rollback/audit horizon |
| security-sensitive | restricted encrypted storage and least-privilege access |

Policies use content references and garbage collection reachability. A baseline/release/accepted decision pins required objects.

# Provenance and Signatures

Local runs rely on digest integrity. CI workers use workload identity to sign final run/release attestations after verifying worker inputs and sealed evidence. Signatures assert provenance, not correctness; correctness comes from referenced deterministic verdicts.

Agent records preserve requested/reported model identity and raw response digest but are not trusted signatures.

# OKF Integration

Accepted run evidence updates OKF through proposal/review:

* contract `verified` event or freshness extension;
* scenario/coverage link changes;
* known-risk/failure concept;
* decision evidence;
* bundle log entry.

The curator emits a patch plus evidence root and exact concept hashes it read. If a concept changed meanwhile, application fails and requires regeneration.

# Artifact Acceptance Tests

* corrupt/missing object prevents replay/certification;
* changing one metadata byte changes root digest;
* derived observations retain raw lineage;
* secret canaries never appear in upload/report/agent inputs;
* replay reproduces verdict byte-for-byte under matching versions;
* unsupported historical oracle yields honest inconclusive;
* concurrent workers cannot overwrite an existing object with different bytes;
* garbage collection retains everything reachable from baselines, releases, and accepted decisions.
