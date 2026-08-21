---
type: Project Overview
title: Terminal Code
description: VS Code inside a terminal by orchestrating terminal-browser and code-server.
tags: [terminal, editor, vscode, code-server]
status: stable
sources:
  - id: readme
    resource: ../../../README.md
    title: Repository README
  - id: cli
    resource: ../../../src/main.ts
    title: Current CLI entry point
---

# Purpose

Terminal Code exposes a `tode` command that opens a VS Code-compatible editor inside a supported terminal. It combines the upstream terminal-browser runtime with code-server rather than reimplementing either upstream product.[^readme]

# Product Characteristics

* Terminal-native graphical editor using the Kitty graphics protocol.
* VS Code-compatible file, folder, goto, add, diff, window reuse, wait, extension, and source-control workflows.
* Automatic terminal palette discovery and a generated editor theme that follows live terminal color changes.
* An interactive shortcut-conflict manager for Ghostty and Kitty.
* Import of settings, keybindings, snippets, tasks, and extensions from VS Code-compatible editors.
* A managed code-server, browser runtime, injector, VS Code bridge extension, and Unix-socket control channel.
* Stable/dev installation, self-upgrade, shutdown, and uninstall workflows.
* A public product site plus embedded import and shortcut-management pages.

# Deployables

1. `tode` application and generated assets.
2. Vendored terminal-browser runtime and pinned code-server.
3. Cloudflare release worker backed by R2.
4. Public website.

# Supported Platforms

macOS and Linux are first-class. Windows is not an official build; the documented path is WSL plus a terminal that implements the Kitty graphics protocol.[^readme]

# Related Knowledge

* [Current architecture](../architecture/current-system.md)
* [Compatibility contract](../contracts/compatibility.md)
* [Rewrite constraints](rewrite-constraints.md)
* [Rust rewrite plan](../plans/rust-rewrite.md)

[^readme]: Repository README
[^cli]: Current CLI entry point
