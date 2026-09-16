# P3D-065 — Integração Paint ↔ UV

<aside>
🧩

Estado: **a implementar (iniciativa Paint · fluxo paint-first, 2026-09-16)** · Prioridade: P1.
Integração bidirecional plena permanece pendente; o Paint consome o utilitário
de projeção UV como "Preparar superfície" até o redesign pós-Paint.

</aside>

## Objetivo

Alternar Paint e UV preservando contexto e permitindo entender imediatamente qual material/texture/channel está sendo editado.

## Fluxo paint-first (decisão 2026-09-16)

Enquanto o workspace UV de edição estiver congelado, o fluxo oficial é
**paint-first**: o Paint oferece as projeções de `module-uv` (Planar, Box/Cúbica,
Auto Unwrap via xatlas) como etapa de preparação da superfície antes da pintura.
A integração completa (alternância bidirecional, seleção compartilhada de
ilhas/faces, undo atravessando workspaces) fica para o redesign pós-Paint,
quando P3D-063 e P3D-064 forem reabertos em conjunto.

## Contrato

Mesmo MaterialId/TextureResource/UVSet; workspace switch não duplica bitmap nem selection. Quando útil, seleção de faces/ilhas pode ser transferida por uma representação explícita.

## Dependências

P3D-050, P3D-055, P3D-063–064.

## Testes / DoD

Alternância repetida, undo atravessando workspaces, save/load e atualizações refletidas no Material Preview.