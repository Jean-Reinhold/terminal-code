---
type: Media Plan
title: Record and Publish Replacement Demo Video
description: Deferred plan for a truthful release-quality demo of the working Rust fork.
tags: [video, website, demo, deferred]
status: draft
blocked_by: [M8, H7]
sources:
  - id: product, resource: ../project/product.md, title: Product capabilities
  - id: contracts, resource: ../contracts/compatibility.md, title: Compatibility contracts
  - id: certification, resource: ../harness/ci-and-platform-matrix.md, title: Release certification gates
---

# Trigger

Record only after the Rust `tode` production entry point passes M8 clean cutover and a current H7/T5 release certificate. The recording must show the shipped fork, not a legacy or partially mocked path.

# Storyboard

1. Install or identify the certified build without exposing credentials or personal paths.
2. Open a folder in the terminal.
3. Reuse the live window to open a file at a line/column.
4. Show a terminal split and source-control review.
5. Demonstrate terminal-derived theme behavior.
6. Resolve one Ghostty or Kitty shortcut conflict.
7. Show a concise import or extension workflow.
8. End on version/build identity and project URL.

Target 45–75 seconds. Every interaction should be real-time product behavior; trim waiting, not outcomes.

# Capture Requirements

* Dedicated clean user/XDG roots and sample repository.
* Supported terminal and Kitty graphics protocol implementation identified in metadata.
* Fixed 16:10 or 4:3 composition readable at website width.
* No tokens, usernames, private paths, notifications, unrelated windows, or personal shell history.
* Record the actual terminal/browser surface at native resolution and stable frame rate.
* Produce one canonical MP4/WebM plus an optimized poster; add a second theme capture only if it demonstrates a real contract difference.
* Provide captions or a concise transcript for accessibility.

# Processing

Use a reviewed Bash or Rust media pipeline around `ffmpeg`: trim, crop, scale, normalize frame rate, remove private audio unless narration is intentional, encode web delivery variants, calculate hashes, and extract posters. Do not add JavaScript/media wrappers merely to process the recording.

# Acceptance

* The certified binary and demonstrated commands match the current release manifest.
* The final video has been reviewed for secrets, accuracy, legibility, pacing, and accessibility.
* Website loading remains lazy and reserves exact aspect ratio.
* Browser checks show no failed media request, layout shift, console error, or inaccessible control.
* README and public site reference the same current asset and replacement date.
* Old demo assets remain deleted.
