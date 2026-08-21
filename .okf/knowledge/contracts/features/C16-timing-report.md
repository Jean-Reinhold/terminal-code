---
type: Compatibility Contract
title: Launch and Workbench Timing Report
contract_id: C16
description: Preserve timing mark capture, stage labels, missing-data behavior, and terminal report formatting.
tags: [timing, performance, browser]
status: draft
risk: medium
owners: [cli, browser]
surfaces: [filesystem, cli, browser]
source_paths: [src/main.ts, src/browserglue.ts, test/browserglue.test.js]
scenario_ids: []
legacy_test_paths: [test/browserglue.test.js]
platforms: [macos, linux]
sources:
  - { id: main, resource: ../../../../src/main.ts, title: Timing report }
  - { id: glue, resource: ../../../../src/browserglue.ts, title: Workbench mark capture }
---

# Contract

Child frames stay out of the timing story; the main workbench records known marks and launch origin. `--timing` alone reads the last record, reports missing data as a zero-exit message, preserves stage labels/order/millisecond formatting and bars, and distinguishes per-open timing when used beside a target.

# Coverage Status

Browser-glue timing tests map here. H3 adds fixed-clock file/CLI golden scenarios.
