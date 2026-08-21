# Compatibility Contracts

* [C01 — CLI identity and basic invocation](C01-cli-identity.md) - Help, version, default target, and basic invocation identity.
* [C02 — Open target and goto parsing](C02-target-and-goto.md) - Existing file/folder, missing file, and goto parsing behavior.
* [C03 — VS Code CLI compatibility flags](C03-cli-compatibility-flags.md) - Ignored and warning-only compatibility options.
* [C04 — Extension management](C04-extension-management.md) - Managed code-server extension operations and profile paths.
* [C05 — Existing window IPC](C05-window-ipc.md) - Unix-socket JSON-line requests, replies, timeouts, and wait.
* [C06 — Startup ordering and onboarding](C06-startup-ordering.md) - Ordered runtime, profile, server, pane, and browser launch.
* [C07 — Runtime and code-server lifecycle](C07-runtime-lifecycle.md) - Pinned verified artifacts, readiness, state, and shutdown.
* [C08 — HTTP and WebSocket injector](C08-http-injector.md) - CSS/font injection, passthrough, readiness, and upgrades.
* [C09 — Terminal palette input](C09-terminal-palette.md) - OSC replies, fallbacks, timing, and live palette propagation.
* [C10 — Theme generation](C10-theme-generation.md) - Color math, contrast, CSS/font, fingerprints, and live application.
* [C11 — JSONC and profile state](C11-jsonc-profile.md) - Source-preserving managed settings and keybindings.
* [C12 — Editor import](C12-editor-import.md) - Compatible editor discovery and user-data migration.
* [C13 — Terminal shortcuts](C13-terminal-shortcuts.md) - Ghostty/Kitty conflict derivation, config, reload, and undo.
* [C14 — Shortcut convergence](C14-shortcut-convergence.md) - Decision state machine, collision safety, and idempotence.
* [C15 — Live window state](C15-live-window-state.md) - Startup marker, theme fan-out, bridge, and socket cleanup.
* [C16 — Timing report](C16-timing-report.md) - Workbench marks and CLI timing presentation.
* [C17 — Command dispatch](C17-command-dispatch.md) - First-argument routing and error/exit behavior.
* [C18 — Release worker HTTP API](C18-release-http.md) - Installer, manifest, download, method, range, and HEAD contracts.
* [C19 — Upgrade transaction](C19-upgrade-transaction.md) - Verified fetch, atomic swap, receipt, and rollback safety.
* [C20 — Uninstall](C20-uninstall.md) - Confirmed cleanup of owned state without user-data escape.
* [C21 — Public site](C21-public-site.md) - Content, responsive visual behavior, metadata, and install proxy.
* [C22 — Embedded pages](C22-embedded-pages.md) - Tokenized import/shortcut UI state and interactions.
