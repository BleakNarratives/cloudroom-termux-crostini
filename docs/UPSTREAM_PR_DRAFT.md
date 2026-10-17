UPSTREAM_PR_DRAFT.md
======================

Target: davidondrej/cloudroom-core

Title:
Add STANK demo (static sidecar) + QRD & Plugin API docs (demo only)

Body:
Hi David,

First — thanks for Cloudroom. Your architecture and documentation made it straightforward for me to experiment with running agent sandboxes locally and building a small demo. This PR adds a tiny, static demo and a couple of documentation artifacts intended to help with UX validation and early-stage plugin design. It is intentionally non-invasive and contains no server-side changes.

What this PR includes
- examples/STANK.html — static conceptual demo (xterm.js) that simulates the cockpit/PTy flow for quick screenshots and UX proofs without needing cross-compiled binaries.
- docs/QRD.md — Quick Rundown template for session handoffs (short-form context + pointers to full logs/embeddings)
- docs/PLUGIN_API.md — initial plugin manifest & runtime contract (WASM-first) for safely extending cockpit features (MrsHiggins, Hotseat, Boardroom examples)

Why this helps
- Provides a low-friction demo for mobile and Chromebook users to preview the interactive shell UX.
- Supplies a handoff QRD standard so operators and agents keep context during asynchronous reviews.
- Starts a plugin API conversation for safe, sandboxed extensions (marketplace/future monetization paths).

Scope & Security
- STANK.html is a static client-side demo only; it does not contact external services or attempt to connect to a running Cloudroom core.
- No changes to core server behavior are included. If maintainers want a real PTY WS integration, I can follow up with a guarded implementation (short-lived token issuance + ws endpoint + PTY spawn) behind a feature flag.

Testing
- Open examples/STANK.html locally in a browser and interact with the simulated terminal and plugin buttons.

Notes & follow-up
If you'd like I can prepare a follow-up PR that implements the PTY WebSocket endpoint (short-lived one-time tokens, proto-binary framing, and a small client-side xterm integration) behind a feature flag. I can also prepare CI cross-compile steps for Termux/Crostini if maintainers want releases for aarch64-linux-android.

Acknowledgements
Thanks again — your videos and docs were very helpful. If you prefer this demo live under examples/ or moved to a demos/ directory, I’m happy to adjust.

— BleakNarratives

