PLUGIN API (Overview)
=======================

Purpose
-------
This document describes a lightweight plugin manifest and runtime contract for Cloudroom sidecar/ cockpit plugins. Plugins are optionally loaded per-session and must be sandboxed. The goal is to allow feature extensibility (MrsHiggins, Hotseat, Boardroom, etc.) while keeping security and resource limits sane.

Plugin manifest (plugin.json)
-----------------------------
Example:

{
  "id": "mrs-higgins",
  "name": "MrsHiggins",
  "version": "0.1.0",
  "entry": "mrs-higgins.wasm",
  "author": "BleakNarratives",
  "description": "Gentle assistant persona for onboarding and hints",
  "capabilities": ["ui-widget", "pty-hook", "agent-adapter"],
  "permissions": ["read:workspace:limited", "emit:events"],
  "signature": "<base64-sig>"
}

Notes:
- entry: path to plugin artifact (WASM preferred). Development plugins can be JS adapters but production should prefer WASM or signed binaries.
- signature: optional signature for plugin verification.

Runtime model
-------------
- The plugin host loads a plugin within a sandbox (WASM runtime or process-isolated adapter).
- Plugins declare capabilities (what they can do) and permissions (what resources they request).
- Plugin lifecycle: install -> enable (per-session) -> run -> disable -> uninstall.
- The host enforces quotas (memory, CPU time) and denies requests exceeding policy.

API primitives (host -> plugin)
-------------------------------
- register_widget(manifest): mounts a UI widget in cockpit
- on_event(event): deliver lifecycle & input events
- request_resource(path, purpose): plugin asks to read a resource; host decides
- emit_metric(name, value): plugin reports telemetry

API primitives (plugin -> host)
-------------------------------
- host.send({type, payload}) // generic RPC
- host.open_stream(name) // request a stream for logs or data
- host.spawn_task(command, args, opts) // strictly limited; host validates against command guard

Events
------
Common events delivered to plugins:
- session.started { session_id }
- session.ended { session_id, exit_code }
- pty.stdout { data }
- pty.stdin { data }
- ui.clicked { widget_id }

Security
--------
- Plugins must declare minimal permissions; host denies unknown permissions
- Prefer readonly workspace access by default
- All plugin network calls must go through host proxies with capability tokens
- Signed plugins are recommended for production

Plugin distribution & marketplace (concept)
-------------------------------------------
- Plugins can be listed with metadata and optional price
- Host verifies plugin signature before install
- Paid plugins are activated via server-side license keys; host stores license and enforces limits

Developer tips
--------------
- Start with a JS adapter in dev: it can register a simple UI widget or respond to pty events.
- Move to WASM for distribution: Rust + wasm-bindgen, or TinyGo/WASI for small runtimes.
- Keep plugin I/O minimal: send events rather than giant payloads.

Example: MrsHiggins behavior
---------------------------
- Capabilities: ui-widget, agent-adapter
- On session.started, MrsHiggins emits a welcome card in the UI
- On pty.stdout matching certain patterns, MrsHiggins offers gentle suggestions
- Runs with memory cap 32MB and CPU cap 50ms/event
