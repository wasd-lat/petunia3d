# Implementation summary

## Implementado

- Command spine + MCP/CLI/FFI/Lua adapters
- TopologyResult, Even Loop Cut, Connect command
- UV seams, islands, pack, project-from-view, texel density, diagnostics
- Paint tiles, lock, screen-space, fill scopes, layer groups, decals
- ZIP `.petunia` + legacy load, GLB import, undo budget, revisions

## Refatorado

- Tools dispatch Commands instead of duplicating checkpoint/mesh ops
- Fingerprint mixes revision counters
- Texture ownership documented as stack → composed cache

## Removido

- Silent unknown primitive → Cone
- MCP private `Project`/`UndoStack`
- Full-scene texture-byte hash as the primary invalidation path

## MVP restante

- GPU query buffer (CPU DTO exists)
- Tile-diff paint undo
- Handle generation (Vec indices + remap)

## MODEL restante

None P0. P2: generational IDs.

## UV restante

None P0. Checker visualization is UI (handoff).

## PAINT restante

P1: GPU query, tile undo diffs. Masks reuse isolation/lock.

## UI handoff

See `ui-handoff.md`.

## Deferred

See `deferred-v1x.md`.

## Testes

TEST EXECUTION INTENTIONALLY DEFERRED BY USER REQUEST.
