# Persistence changes

- Save: ZIP V1 (`manifest.json`, `document.json`) via `encode_zip` (no extra Project clone for postcard envelope)
- Load: ZIP or legacy postcard `PETUNIA\0`
- Limits: 256 MiB file / uncompressed, 4096 zip entries, path traversal rejected
- Autosave: still atomic; `capture_snapshot` for worker hand-off
- Undo: byte budget 256 MiB, oldest-first eviction, `HistoryMetrics`
