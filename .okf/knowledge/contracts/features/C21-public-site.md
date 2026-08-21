---
type: Compatibility Contract
title: Public Product Site
contract_id: C21
description: Preserve public content, responsive visual system, assets, metadata, install flow, video, analytics, and release proxy.
tags: [web, public-site, visual, install]
status: draft
risk: medium
owners: [web]
surfaces: [browser, http, visual, accessibility]
source_paths: [web/app/page.tsx, web/app/layout.tsx, web/app/globals.css, web/app/components, web/next.config.ts]
scenario_ids: []
legacy_test_paths: []
platforms: [browser]
sources:
  - { id: page, resource: ../../../../web/app/page.tsx, title: Public page }
  - { id: styles, resource: ../../../../web/app/globals.css, title: Public visual system }
  - { id: config, resource: ../../../../web/next.config.ts, title: Install proxy }
---

# Contract

Preserve page copy/structure, responsive rail/content layout, fonts/colors/rules/assets, video controls, usage/install interactions, header/footer/GitHub links, metadata/opengraph/analytics, accessibility/focus behavior, and `/install` proxy semantics that keep large downloads off site bandwidth.

# Coverage Status

No current browser suite exists. H5 requires Rust-driven real-browser DOM/accessibility/network/visual scenarios before replacing Next.js.
