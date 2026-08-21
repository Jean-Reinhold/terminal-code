---
type: Planning Procedure
title: Rust Rewrite Planning Swarm
description: Independent evidence workstreams and the blocked execution record.
tags: [planning, swarm, rust, blocked]
status: draft
sources:
  - id: architecture
    resource: ../architecture/current-system.md
    title: Current system architecture
  - id: contracts
    resource: ../contracts/compatibility.md
    title: Compatibility matrix
---

# Intended Workstreams

1. CLI invocation and compatibility parsing.
2. code-server and terminal-browser runtime lifecycle.
3. bridge, IPC, browser glue, OSC, and live-sync protocols.
4. shortcut conflict domain, Ghostty/Kitty adapters, and manager UI.
5. import, profile, JSONC, theme, and onboarding state.
6. installer, dist, upgrade, uninstall, release worker, and publication.
7. public and embedded web surfaces.
8. legacy-vs-Rust parity oracle and coverage gaps.
9. Cargo workspace and dependency architecture.
10. migration, security, CI, release, rollback, and deletion governance.

# Common Output Contract

Every planner must return:

* observed features with exact path/symbol/test/config evidence;
* external contracts and compatibility requirements;
* proposed Rust crates/modules and dependency choices;
* ordered, independently gateable migration steps;
* observable parity checks;
* decisions and rejected alternatives;
* risks with mitigations;
* unresolved facts rather than invented assumptions.

Planners are read-only and must not build, test, lint, format, edit, install, or run services. Current-state claims without repository evidence are rejected.

# Execution Record

Requested backends: Opus 5 initially; DeepSeek V4 Flash for the harness-planning retry.

Observed runtime constraints:

* The task API exposed agent roles but no model selector, so the requested model could not be guaranteed.
* Ten parallel generic planning workers were launched. Every worker failed before repository access with an expired Anthropic OAuth session and rejected beta headers.
* Ten parallel Codex-rescue workers were then launched as a fallback. Their Claude worker shells failed with the same authentication error before invoking Codex.
* Eight harness-planning workstreams were routed with `--model deepseek-v4-flash`. Every codex-rescue worker failed in the same Anthropic wrapper before the downstream model invocation, so DeepSeek V4 Flash was never reached.
* A downstream model choice cannot bypass the failed worker preflight; successful model provenance must come from the provider response, not the route request.
* No worker produced repository analysis. No local conclusion in this bundle is presented as swarm output.

# Status and Retry Gate

This procedure is blocked until the Anthropic worker OAuth session and beta-header configuration are repaired. Model routing alone does not unblock it. After repair:

1. Re-run all ten workstreams concurrently against the same clean commit.
2. Store raw outputs as review artifacts, not as authoritative concepts.
3. Verify every cited path/symbol against the indexed repository.
4. Reconcile conflicts into proposed changes to the target architecture, compatibility matrix, ADRs, and milestone plan.
5. Record accepted/rejected planner recommendations and human review in the bundle log.

The first-party [Rust rewrite plan](rust-rewrite.md) is actionable but remains `draft` until that review or an equivalent human architecture review occurs.
