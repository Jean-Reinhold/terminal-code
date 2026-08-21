---
type: Operations Playbook
title: Rust Release and Supply Chain
description: Target release transaction preserving current artifact, install, upgrade, and rollback behavior.
tags: [operations, release, supply-chain, rust]
status: draft
sources:
  - id: dist
    resource: ../../../scripts/dist.sh
    title: Current local distribution layout
  - id: release
    resource: ../../../scripts/release.sh
    title: Current release builder
  - id: publish
    resource: ../../../scripts/publish-r2.sh
    title: Current R2 publication
  - id: worker
    resource: ../../../release-worker/src/index.ts
    title: Current release service
---

# Invariants

* Stable and dev are separate channels.
* Every target artifact has a filename, byte size, and SHA-256 in one versioned manifest.
* The installed tree is staged beside the live tree and atomically renamed into place.
* A failed fetch, verification, unpack, or swap leaves the working installation intact.
* First launch can use the vendored terminal-browser without network access.
* Pinned-version and latest manifests remain addressable through current URLs.
* Rollback installs a complete prior artifact; it never copies selected files backward.

# Target Release Transaction

`cargo xtask release <version> <channel>` performs one explicit transaction:

1. Refuse a dirty tree and validate the semantic version/channel.
2. Build native `tode`/installer artifacts and all required WASM targets from the locked commit.
3. Resolve the pinned terminal-browser and code-server artifacts, verify their provenance, and stage the existing install layout.
4. Generate pinned VS Code keymaps from the pinned code-server version.
5. Build deterministic per-target archives.
6. Re-open each archive, validate required paths/modes, then calculate size and SHA-256 from final bytes.
7. Emit the versioned manifest and installer/bootstrap payloads.
8. Install every artifact into a clean sandbox, launch offline, upgrade, roll back, and uninstall.
9. Upload immutable versioned objects into a non-production namespace and read back their bytes.
10. Deploy the Rust worker and public site to staging.
11. Run the full T5 harness certification: route/range/HEAD/cache, install/offline launch, upgrade interruption, rollback, uninstall, platform/hardware freshness, security, and required agent review.
12. Seal the evidence root and sign a release certificate binding source, builds, archives, manifest, platforms, contracts, and rollback artifact.
13. Verify the certificate from an independent clean verifier.
14. Upload/read back immutable production objects and versioned manifests/installers.
15. Update `<channel>/latest.json` last with compare-and-swap, accepting only the certified exact manifest/artifact digests.
16. Run post-publication worker/site/install smoke checks.
17. Record commit, artifacts, hashes, manifest, evidence root, certificate, rollback target, and smoke evidence.

A failure before step 15 leaves the previous latest release active. A failure after step 15 triggers latest-pointer rollback to the previous complete certified manifest. See the [harness CI and platform gates](../harness/ci-and-platform-matrix.md) and [evidence model](../harness/evidence-and-artifacts.md).

# Artifact Layout

Preserve the user-visible install locations while replacing internals:

```text
VERSION
CHANNEL
bin/tode
assets/
vendor/terminal-browser/
vendor/code-server/          # if the current release stages it here; freeze exact layout in M0
```

The Rust binary replaces `dist/main.js` and Electron-as-Node shims. The final exact archive tree is frozen from the current release before implementation.

# Installer and Upgrade

The bootstrap selects the current `darwin|linux` and `arm64|x64` naming contract, downloads the Rust installer, verifies declared size/SHA-256, and executes it. The Rust installer owns archive verification, staging, receipts, shim creation, and atomic swap.

Upgrade retains current outcomes: not an install, current, available under `--check`, and upgraded. It writes version/channel receipts only after a successful swap and stops the old managed server after the new tree is durable.

# Worker and Site

The Rust/WASM worker retains [release routes and schemas](../contracts/state-and-protocols.md). It streams R2 objects without buffering full ~130 MB artifacts, preserves conditional/range/HEAD behavior, and rejects path traversal or mismatched manifest filenames.

The public site preserves `/install` proxy behavior or replaces it with an equivalent edge route where download URLs still point directly to the release worker/R2 path rather than consuming site bandwidth.

# Supply-Chain Baseline

The rewrite preserves current size plus SHA-256 verification. Signing, transparency logs, or a new archive format are valuable but are separate product decisions after parity; adding them inside the rewrite would change scope and complicate rollback compatibility.

Required controls now:

* locked Rust dependencies and reviewed license/advisory policy;
* pinned GitHub Actions and toolchain inputs;
* no secrets in artifacts/logs;
* least-privilege R2/worker publication credentials;
* immutable version objects;
* manifest filename/path validation;
* checksum verification before unpack;
* archive traversal/symlink escape rejection;
* reproducible generation of host loaders and public static output.

# Rollback

Keep the previous latest manifest and artifacts immutable. Rollback changes only the channel latest pointer after smoke-testing the prior installer/manifest. Users already upgraded roll back through the same verified installer path; operators do not mutate installed trees manually.
