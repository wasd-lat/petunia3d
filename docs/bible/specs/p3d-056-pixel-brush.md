# P3D-056 — Pixel Brush

<aside>
🧩

Estado: **em implementação (iniciativa Paint, 2026-09-16)** · Prioridade: P1.
Consumidor do novo descriptor `BrushSettings` (ver P3D-055).

</aside>

## Objetivo

Brush pixel-oriented com bordas/densidade adequadas a texturas retro/low-poly.

## BrushSettings (decisão 2026-09-16)

O Pixel Brush passa a ser configurado exclusivamente pelo descriptor unificado
`BrushSettings` — `size_px`, `hardness`, `strength`, `flow`, `spacing`. A
dualidade anterior `canvas_brush` (px) × `paint_radius` (metros) é deprecada;
tamanho é sempre em **pixels de tela** (estilo Photoshop), consistente com o
Soft Brush, Eraser e o novo Airbrush. A diferença do Pixel Brush fica no
sampling/filter/shape (nearest, sem suavização), não em parâmetros próprios.

## Arquitetura

Reutilizar stroke engine comum; diferença do Pixel Brush deve ser sampling/filter/shape, não um pipeline separado. Integrar active layer, mask e channel.

## UX

Nearest/pixel behavior previsível, size em pixels/texels quando fizer sentido, preview do cursor e sem suavização inesperada.

## Dependências

P3D-055, P3D-061, P3D-132.

## Testes / DoD

Diferentes zooms/resoluções, UV seams, masks, undo e pixel-perfect quando prometido.