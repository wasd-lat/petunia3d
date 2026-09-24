# P3D-027 — Pivot System

<aside>
🧩

Estado: **parcialmente implementado; auditar política e casos de seleção** · Prioridade: P0.

</aside>

## Objetivo

Definir pivô de transformação previsível para objeto/componentes e contexto da cena.

## Auditoria

Mapear pivôs atuais, median/center/origin/cursor equivalentes e divergências entre Move/Rotate/Scale.

## Arquitetura

Pivot policy é estado semântico do editor. UI mostra selector; transform core recebe pivot calculado de forma determinística.

## Dependências

P3D-021–026, P3D-049.

## Testes / DoD

Seleção simples/múltipla, origem do objeto, median/center aprovados, pivot após delete/merge e persistência apenas quando apropriada.

## Estado da UI Slint — 2026-09-23

O shell Slint de produção expõe um selector no header da viewport para `Median
Point`, `Bounding Box Center`, `3D Cursor` e `Individual Origins`. A escolha
atualiza `EditorSession::pivot_point`, que o modal de Move/Rotate/Scale já lê.
Isso fecha o gap de descobribilidade; não certifica os resultados matemáticos
para seleção múltipla, componentes, active object após delete/merge ou
persistência. Esses casos permanecem sob a auditoria P3D-027.
