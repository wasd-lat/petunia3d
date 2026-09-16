# P3D-064 — UV Editing básico

<aside>
🧩

Estado: **redesign pós-Paint (2026-09-16)** · Prioridade: P1.
A edição UV sai da UI V1 durante a iniciativa Paint; o utilitário de projeção
permanece em `module-uv` e é consumido pelo Paint como "Preparar superfície".

</aside>

## Objetivo

Operações essenciais de UV compatíveis com o workflow low-poly.

## Decisão 2026-09-16 — iniciativa Paint

O escopo de edição UV (ilhas, seams, Move/Rotate/Scale, mark/unmark seam,
packing) fica suspenso na UI V1 enquanto a iniciativa Paint não estabilizar o
fluxo paint-first. A utilidade do módulo `module-uv` (Planar/Box/Auto Unwrap)
é preservada e exposta dentro do Paint para preparar a superfície antes de
pintar. O redesign completo do editor UV volta no ciclo pós-Paint, em conjunto
com P3D-063 e P3D-065.

## Escopo inicial sugerido

Visualizar ilhas, selecionar, Move/Rotate/Scale UV, mark/unmark seam quando existir suporte, unwrap básico e packing apenas se a implementação permanecer simples/previsível.

## Regras

Não prometer smart unwrap avançado. Preservar correspondência vertex/loop/UV com alterações topológicas suportadas.

## Dependências

P3D-063, P3D-065, P3D-050.

## Testes / DoD

Meshes simples, seams, múltiplas ilhas, overlap permitido/detectado conforme política, undo e serialization.