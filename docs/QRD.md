Quick Rundown (QRD) Template
=============================

Purpose
-------
QRD.md provides a compact, copy-paste-friendly summary for a session or room that an operator or another agent can quickly read to understand context, state, and next steps.

Use this template inside docs/ or attach to session artifacts in storage. Keep it short — a few lines per field — and provide pointers to full logs and embeddings.

Template
--------

Session ID: <session_id>
Harness: <harness_name>
Created at: <ISO timestamp>
Operator: <operator username or agent id>
Permissions: <read/write/list of allowed resources>

Last input (first 240 chars):
> <short snippet>

Last output (first 240 chars):
> <short snippet>

State: <active|idle|terminated|error>
Exit code (if any): <code or N/A>

Attachments:
- path/to/file1 (gz)
- s3://bucket/path/to/artifact

Embeddings refs (if used):
- embedding_id: <uuid> -> /store/embeddings/<uuid>

Pointers:
- Full log: /path/to/log.gz
- Snapshot: /path/to/snapshot.tar.gz

Recommended follow-ups (operator guidance):
1. Inspect file /workspace/xyz for suspicious changes
2. Run `files check` harness with seed abc123
3. If no activity for 10m, archive session

Notes & context (freeform):
- Short notes, sources, or URLs the operator should know.

Handoff checklist (one-line items):
- [ ] Confirm token validity (expires at <timestamp>)
- [ ] Ensure plugin permissions are correct
- [ ] Confirm storage has > 200MB free

Example filled snippet
-----------------------
Session ID: s-7f4c9a
Harness: hermes
Created at: 2026-10-15T04:12:06Z
Operator: bleak
Permissions: read:workspace, spawn:limited

Last input:
> Run quick scout on repo and look for TODO comments

Last output:
> Scout found 3 TODOs in src/lib.rs and tests/flow_test.rs

State: active
Attachments:
- /store/sessions/s-7f4c9a/logs/stream.1.gz
- /store/sessions/s-7f4c9a/artifacts/snapshot.tar.gz

Recommended follow-ups:
1. Run `scout --deep` with timeout 30s
2. Archive session if no hits after next run


Why QRD is useful
-----------------
- Avoids dropped context during human handoff
- Keeps handoff packets short for async review
- Embeds pointers to full data if someone needs to deep-dive
