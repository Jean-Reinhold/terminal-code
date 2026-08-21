---
type: Compatibility Contract
title: Embedded Import and Shortcut Pages
contract_id: C22
description: Preserve tokenized local pages, state rendering, actions, progress, apply, cancel, completion, and navigation.
tags: [web, embedded, import, shortcuts]
status: draft
risk: high
owners: [web, import, shortcuts]
surfaces: [browser, http, filesystem]
source_paths: [src/pages/import/app.tsx, src/pages/shortcuts/app.tsx, src/webui/pages.ts, src/webui/tokens.ts, src/import/web.ts, src/shortcuts/web.ts, test/import.test.js]
scenario_ids: []
legacy_test_paths: [test/import.test.js]
platforms: [macos, linux, browser]
sources:
  - { id: import-page, resource: ../../../../src/pages/import/app.tsx, title: Import page }
  - { id: shortcut-page, resource: ../../../../src/pages/shortcuts/app.tsx, title: Shortcut page }
  - { id: server, resource: ../../../../src/webui/pages.ts, title: Local page server }
---

# Contract

Serve reviewed built pages only through unguessable scoped tokens, reject invalid/expired access, preserve initial/session state, editor selection, import progress/report/done flow, shortcut row/claim/collision decisions, free-chord feedback, apply/cancel/confirmation, reload/navigation, responsive styling, keyboard operation, and accessible state.

# Coverage Status

The import page server test maps here; shortcut UI has no browser test. H5 requires Rust-driven real-browser scenarios before React/Vite removal.
