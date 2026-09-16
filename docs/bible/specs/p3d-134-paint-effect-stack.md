# P3D-134 — Paint Effect Stack

<aside>
🧩

Estado: **em implementação (iniciativa Paint, 2026-09-16)** · Prioridade: P2.
UX de presets "Add Effect" na pilha de camadas; nodes iniciais do capítulo 42.

</aside>

## Objetivo

Efeitos não destrutivos simples sobre layers/texturas.

## UX de presets "Add Effect" (decisão 2026-09-16)

A pilha de camadas do Paint oferece um botão **"Add Effect"** que abre a lista
de presets disponíveis. O usuário escolhe o efeito e ele é inserido na pilha
na posição atual, reorderável por drag-and-drop. Cada efeito é um node com
parâmetros próprios e pode ser habilitado/desabilitado individualmente.

## Nodes iniciais (capítulo 42)

Lista consolidada de efeitos não destrutivos a implementar, em ordem de
prioridade da iniciativa Paint:

| Node | Status |
| --- | --- |
| Pixelate | já existe no modelo |
| Posterize / quantize | já existe no modelo |
| Invert | já existe no modelo |
| Grain / Noise | a implementar |
| Levels / Threshold | a implementar |
| Brightness / Contrast | a implementar |
| Hue / Saturation | a implementar |

Aquarela/lápis/caneta só entram após protótipos de qualidade aceitável e custo conhecido.

## Regra conceitual

Diferenciar **efeito sobre textura** de **shader toon em tempo real** (P3D-140).

## Arquitetura

Effect descriptors serializáveis, evaluation cacheable e sem filtro pesado refeito por frame sem necessidade. O modelo de dados é o Surface Recipe graph (P3D-113): DAG, sockets tipados, avaliador determinístico e cache por node. Nesta fase o graph é **headless** — o editor visual de nodes fica para o ciclo pós-Paint.

## Dependências

P3D-055, P3D-061, P3D-113.

## Testes / DoD

Reorder effects, parameters, disable/enable, serialization, invalid values e performance.