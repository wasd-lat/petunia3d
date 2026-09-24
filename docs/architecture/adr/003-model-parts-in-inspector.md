# ADR 003 — Parts no Inspector de MODEL

Status: aceito em 2026-09-23 por autorização explícita do responsável do produto.
Autoridade: [Livro Vivo, capítulo 36](../../bible/foundations/36-ui-baseline-temas-plugin-panels.md), revisão de baseline MODEL de 2026-09-23. Este ADR registra o motivo; não cria uma segunda baseline.

## Contexto

A gaveta Parts à esquerda cobria a barra de primitivas e dividia a hierarquia de
MODEL em duas regiões concorrentes. O Inspector direito não tinha rolagem efetiva
nem sequência estável de seções. Em telas compactas, ocultar o Inspector sem um
acesso alternativo deixaria a árvore de peças indisponível.

## Decisão

No workspace MODEL, o trilho esquerdo concentra criação; a coluna direita
contém um Inspector com cabeçalho fixo e seções `Parts → Transform → Material →
Object`. Duplicate/Delete pertencem a Object. O corpo rola verticalmente; Parts
usa lista virtualizada, busca, filtro, ordenação, visibilidade, lock e tamanho de
linha. Busca/filtro/ordenação/contagem ocupam uma faixa compacta compartilhada; o
controle de tamanho atualiza a altura das linhas nos layouts normal e compacto.
Cada seção recolhe sem reservar espaço e mantém o estado ao alternar de
workspace. Abaixo de 900 logical px, um drawer controlado à direita preserva o
acesso à lista sem cobrir a barra de criação. O comando legado de abrir Parts
permanece como alias de acesso.

## Consequências

- A antiga regra `Parts` à esquerda no capítulo 36 foi substituída apenas para MODEL.
- Slots de extensão continuam controlados; plugins não podem mover as quatro seções centrais.
- PAINT e UV não são redesenhados por este ADR.
- Parâmetros de primitiva pós-criação requerem persistência do descritor no modelo
  e transação de regeneração; controles sem esse contrato seriam enganosos.
- Aceite exige viewport mínima preservada, teclado/tooltip, testes de lista e
  gaveta compacta, captura nativa e gates de CI.
