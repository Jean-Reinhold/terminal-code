---
type: Architecture Decision
title: Clean Rust Cutover
description: Use the legacy implementation as a temporary oracle and remove it at production cutover.
tags: [adr, rust, migration, cutover]
status: draft
sources:
  - id: constraints
    resource: ../project/rewrite-constraints.md
    title: Rewrite constraints
  - id: plan
    resource: ../plans/rust-rewrite.md
    title: Rust rewrite plan
---

# Context

A gradual rewrite needs the current implementation to establish behavior, but shipping two production implementations creates drift, doubles release/security work, and leaves ownership ambiguous.

# Decision

The legacy system remains executable only for compatibility fixtures during migration. Production continues using the complete legacy release until the Rust release passes the clean-cutover gate. The cutover switches the entire production entry point and then deletes the legacy application/build/release path in the same change.

Temporary dual-run is permitted only in isolated tests. Real user state is never written by both implementations in one scenario.

# Consequences

Positive:

* One authoritative production implementation before and after cutover.
* Differential evidence without permanent compatibility shims.
* Rollback uses a complete prior release rather than mixed-language files.

Negative:

* The final cutover is larger than command-by-command production switching.
* Contract fixtures and release-candidate coverage must be strong before switching.
* Feature work should be frozen or applied to both branches during the migration window to avoid moving parity targets.

# Rejected Alternatives

* Permanent sidecar/FFI bridge: retains two runtimes and failure domains.
* Command-by-command production routing: risks shared-state corruption and inconsistent error/output behavior.
* Rewrite with behavior cleanup at the same time: prevents differential verification and expands scope.

# Acceptance

Promote this decision to stable only when release rollback, compatibility freeze ownership, and the M8 deletion checklist have named maintainers.
