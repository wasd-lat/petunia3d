# P3D-063 — UV Workspace

<aside>
🧩

Estado: **edição congelada durante a iniciativa Paint (2026-09-16)** · Prioridade: P1.
Redesign marcado para **pós-Paint**.

</aside>

## Objetivo

Workspace dedicado a visualizar/editar UV sem duplicar dados do Paint/Material.

## Decisão 2026-09-16 — iniciativa Paint

A aba/superfície de **edição UV** sai da UI V1 durante a iniciativa Paint. Motivo:
conflito de seleção compartilhada — o `uv_ui` alterna `f.selected`, a mesma flag
usada pelas máscaras de pintura (P3D-132) — e P3D-063 já exigia "design conjunto
com Materials/Paint". O workspace UV como pill independente volta no redesign
pós-Paint, quando a integração com Paint e Materials estiver madura.

O **módulo `module-uv` permanece** como utilitário: projeção Planar, Box/Cúbica
e Auto Unwrap (xatlas) passam a ser oferecidas como "Preparar superfície" dentro
do workspace Paint, porque pintar depende de UV projetada.

## UI

Viewport UV 2D, seleção sincronizável quando útil, toolbar/shelf contextual e acesso claro à texture/channel ativo. Não manter timeline ou painéis irrelevantes por padrão.

## Arquitetura

UV data pertence à mesh; texture/material state é compartilhado com P3D-050/055. Workspace é apenas composição de views/tools.

## Dependências

P3D-050, P3D-055, P3D-064, P3D-065.

## Testes / DoD

Abrir/fechar workspace preserva seleção/material, DPI/responsive e nenhum dado é copiado para estado exclusivo da UI.