---
type: Architecture Decision
title: Deterministic Verdicts, Agentic Discovery
description: Agents expand and review evidence; deterministic Rust code alone executes scenarios and decides gates.
tags: [adr, harness, agents, verification]
status: draft
sources:
  - id: principles
    resource: ../harness/principles.md
    title: Harness principles
  - id: orchestration
    resource: ../harness/agent-orchestration.md
    title: Agent orchestration
  - id: evidence
    resource: ../harness/evidence-and-artifacts.md
    title: Evidence model
---

# Context

Agents are strong at semantic exploration, case generation, adversarial thinking, visual review, and failure explanation. They are nondeterministic, provider-dependent, vulnerable to untrusted repository/evidence instructions, and can be unavailable—as demonstrated by planning workers failing before model invocation.

A release gate must remain reproducible and inspectable without trusting a model opinion.

# Decision

Use agents for impact proposals, coverage planning, scenario/fixture proposals, adversarial variants, visual/evidence review, triage, skeptical review, and OKF patch proposals.

Use deterministic Rust code for schema/policy validation, sandbox provisioning, execution, observation, normalization, assertions, verdict aggregation, baseline application, CI gates, and release certification.

Agents can broaden deterministic scenario selection but cannot remove it. Agents cannot modify approved contracts, risk, scenarios, normalizers, baselines, evidence, verdicts, or stable OKF status directly.

A named-model requirement passes only when invocation provenance reports that model. A model name embedded in a prompt or downstream route request is not evidence that the model ran.

# Consequences

Positive:

* Known suites and replay work during complete provider outage.
* Agent contributions are aggressive without becoming a correctness root.
* Model/provider changes do not alter historical verdict semantics.
* Prompt injection and fabricated citations are constrained to rejected proposals.

Negative:

* Agent outputs require schemas, evidence checks, and deterministic admission.
* Some qualitative visual/contract ambiguity remains human-review-gated.
* More infrastructure is required than a free-form agent test loop.

# Rejected Alternatives

* LLM-as-judge pass/fail: cannot be replayed deterministically and can bless missing evidence.
* Majority-agent vote: correlated models/prompts are not independent proof and cannot override contracts.
* Fully deterministic test selection only: loses valuable semantic impact/adversarial discovery.
* Automatic agent baseline updates: converts implementation drift into accepted behavior without authority.

# Acceptance

Stabilize only after the harness proves provider outage, prompt injection, fabricated evidence, model mismatch, and malicious baseline proposal cannot create a passing deterministic gate.
