# Plano de implementação — Context, workspaces e viewport

Estado: **planejado, não implementado**. Reconciliação de 23/09/2026. Este plano
organiza a proposta de refinamento visual recebida do usuário; não altera o
Livro Vivo nem declara paridade alcançada. A próxima discussão e execução ficam
restritas a **MODEL e viewport**. PAINT e UV aparecem aqui apenas para que a
base compartilhada não precise ser refeita depois.

## Autoridade e decisões pendentes

O capítulo [36](../bible/foundations/36-ui-baseline-temas-plugin-panels.md)
congela o shell viewport-first, `Parts`/`Context`/`Asset Library`, viewport de
aproximadamente 480 × 360 logical px e limites de 240–440 px para Context.
Os capítulos [23](../bible/foundations/23-macroarquitetura-interface.md) e
[24](../bible/foundations/24-design-system-tokens-estados.md) exigem contexto
centrado na seleção, componentes e tokens semânticos. A implementação é Slint;
`crates/ui/` é legado e não recebe novas features de produto.

| Proposta recebida | Reconciliação necessária antes de alterar o produto |
|---|---|
| Mostrar `Inspector` nos três modos | `Context` é o nome público canônico (§23/36). `InspectorShell` pode ser nome interno; trocar o rótulo visível exige decisão no Livro Vivo. Até lá, preservar `Context` e padronizar o header/indicador do workspace. |
| `Tool Options` como primeira seção do Inspector | [P3D-083](../bible/specs/p3d-083-tool-properties.md) separa parâmetros da ferramenta das propriedades do objeto. O shell pode hospedar um slot contextual visualmente coerente, mas com descriptor/estado próprios; não misturá-los ao Object Inspector. |
| UV central 3D/2D como fase imediata | A baseline prevê split 55/45, porém o adendo de 16/09 no §36 congela temporariamente a pill UV durante a iniciativa Paint. Planejar o split, mas não tratá-lo como autorização para expandir UV antes da reconciliação canônica. O código atual exibe a pill: drift conhecido. |
| Quatro shadings incluindo `Rendered` | O [capítulo 05](../bible/foundations/05-viewport-shading-modos-visualizacao.md) define Wireframe, Solid, Textured e Silhouette/Reference, sem Rendered como modo principal. O pedido anterior de Rendered permanece divergência aberta; nenhuma troca silenciosa. |
| Context 320–360 px, header 34–36 px, escala 80/90% | Usar 320–360 como hipótese de tuning dentro dos 240–440 px; preservar o header de referência 28 px e presets oficiais 100–200% até teste/decisão canônica. Não copiar os hex propostos literalmente. |
| Esquerda = ferramentas; topo = visualização; base = feedback | A rail esquerda atual é coluna estrutural de 40 px, enquanto §23/36 exigem toolbar contextual **dentro** da viewport. O §23 também reserva à viewport bar áreas de contexto/transformação/snap/pivô. Tratar a proposta como hipótese de reorganização, não como autorização para deixar o topo somente visual. |

## Implementation-vs-Spec Gap Matrix

Evidência primária: `crates/ui-slint/ui/app.slint`, `tokens.slint`, bridge em
`crates/ui-slint/src/lib.rs`, estado de UI em `crates/core/src/state.rs` e
[matriz da viewport](viewport-gap-matrix.md). A classificação abaixo se refere
ao comportamento observável, não apenas à existência de um símbolo.

| Área | Estado | Evidência / delta |
|---|---|---|
| Shell viewport-first e largura do Context | PARTIALLY_COMPLIANT | Divisor e largura por workspace já existem; o corte implementado é `<1100` contra `<1024` da baseline, e Context some sem drawer, ocultando os editores 2D. Não refazer o divisor; medir mínimo da janela antes de alterar o breakpoint. |
| Header fixo e corpo rolável do Context | BROKEN | `VerticalLayout` contém header e todas as seções sem `ScrollView`; PAINT corta controles e a altura preferida pode pressionar o shell. |
| `InspectorSection` reutilizável | PARTIALLY_COMPLIANT | Componente e clique no cabeçalho já existem. O corpo usa `visible: root.open`, os `open` são locais, não há persistência/Alt+click/foco completo; `accessible-role: button` está no contêiner de campos aninhados, não só no header. Medir se o corpo fechado ainda reserva espaço. |
| MODEL — Transform e operações | PARTIALLY_COMPLIANT | `Vector3Field` e intents transacionais funcionam; Object/Material são esparsos, faltam grupos Selection/Geometry/Statistics úteis e Duplicate/Delete ocupam um rodapé desproporcional. |
| Tool Properties | RUDIMENTARY | Painel flutuante modal/Loop Cut existe, mas os controles são hardcoded e falta descriptor estável de parâmetros de ferramenta exigido por P3D-083; não criar segunda fonte editável no Context. |
| PAINT — ordem, rolagem, Layers | RUDIMENTARY | Ordem atual começa por Effects/Canvas; Canvas ocupa 200 px no Context; Brush fica ao fim; não há `ListView` para camadas. Planejamento apenas nesta rodada. |
| UV — editor e Inspector | FUNCTIONAL_BUT_DIFFERENT | O editor UV está numa seção de 256 px do Context, não na região central; há conflito de escopo com o adendo canônico. Planejamento pós-decisão. |
| Barras da viewport | DUPLICATED | Rail estrutural esquerda e barra flutuante inferior repetem Select/Move/Rotate/Scale e ferramentas por workspace. A barra superior já contém controles de visão. O posicionamento da rail também diverge da toolbar contextual dentro da viewport prevista no §23/36. |
| Feedback contextual | PARTIALLY_COMPLIANT | Status/HUD modal existem, mas a barra inferior ainda compete com ferramentas; não substituir feedback por mais botões. |
| Tokens, estados e acessibilidade | PARTIALLY_COMPLIANT | `DesignTokens` e papéis acessíveis existem; há medidas, strings e ícones inline, sem contrato completo de foco/hover/disabled nem escala global de UI. |

## Arquitetura-alvo incremental

```text
Shell comum (Slint)
├─ Top Bar
├─ Parts / toolbar contextual dentro da viewport (posição a reconciliar)
├─ Centro: viewport 3D; editor 2D opcional somente quando autorizado
├─ ContextShell
│  ├─ header fixo + workspace atual
│  ├─ região Tool Properties independente (descriptor próprio, P3D-083)
│  └─ ScrollView vertical, altura limitada
│     └─ seções compartilhadas, contextualizadas por seleção/workspace
├─ Asset Library retrátil
└─ status e HUD contextual
```

`ContextShell`, `InspectorSection`, campos, linhas, botões e listas compartilham
contratos de propriedades/callbacks/estados. Isso não implica três sistemas
independentes nem reescrita completa de `app.slint`: extrair e validar uma peça
por vez. Mutação continua em `UiIntent → Command → Algorithm → Data`; layout e
estado de seção pertencem à sessão de UI, não ao documento `.petunia`.
Tool Properties tem owner de estado/descriptor separado do Object Inspector e
um único local editável por vez: painel flutuante **ou** região contextual,
sem cópias simultâneas. Os extension slots de plugins à direita permanecem
controlados pelo shell, sem acesso cru à árvore Slint.

Para o corpo, usar o [ScrollView oficial](https://docs.slint.dev/latest/docs/slint/reference/std-widgets/views/scrollview/)
com altura limitada pelo layout, `vertical-scrollbar-policy: as-needed` e
`horizontal-scrollbar-policy: always-off`. A API atual chama a dimensão do
conteúdo de `content-height` (`viewport-height` é alias depreciado); conferir
o comportamento na versão fixada Slint 1.18 ao implementar. O ScrollView cria
todos os filhos; coleções numerosas devem usar
[ListView](https://docs.slint.dev/latest/docs/slint/reference/std-widgets/views/listview/),
que instancia somente itens visíveis. `Palette` e
[StyleMetrics](https://docs.slint.dev/latest/docs/slint/reference/std-widgets/globals/stylemetrics/)
são referências para consistência de widgets; não substituem o
`DesignTokens`/`ThemeToken` canônico do produto.

## Fases e critérios de aceite

| Fase | Escopo e dependência | Entrega verificável |
|---|---|---|
| 0 — Baseline e decisões | Fotografar MODEL/PAINT/UV, medir geometria/overflow/escala e registrar divergências acima. | Matriz atualizada; nenhuma mudança de nome, modo ou UV feita por inferência. |
| 1 — ContextShell | Header fixo, corpo rolável, largura limitada, drawer compacto autorizado, sem scroll horizontal. Preservar divisor, largura por workspace e extension slots. | Em 1023/1024/1099/1100 px, 1280×800 e 1920×1080, nenhuma seção passa da janela; viewport mantém ~480×360; scroll alcança a última seção; Asset Library/drawer seguem breakpoints canônicos; sem expansão involuntária da janela ou ciclo de binding. |
| 2 — InspectorSection | Consolidar o componente atual; corpo fechado sai do layout, cabeçalho inteiro ativável, múltiplos abertos, foco/teclado, estado por workspace. Alt+click de todos somente via input/keymap aprovado. | Testes de geometria, foco, estado expandido em AccessKit e persistência de sessão; mudar de workspace e voltar restaura cada grupo sem alterar Undo do documento, inclusive durante criação de primitiva/modal. |
| 3 — MODEL Context | Compor Selection/Transform/Geometry/Normals & Shading/Material/Object Data/Statistics **somente quando relevantes à seleção/tarefa**; Tool Properties fica em região independente. Remover botões grandes do rodapé e preservar Duplicate/Delete via comando/menu contextual. | Sem seções vazias ou controles placebo; X/Y/Z de largura igual, rótulos além da cor; seleção e parâmetros mudam sem estado visual residual; operações conservam Undo/cancel. |
| 4 — Viewport MODEL | Deduplicar rail/barra inferior sem perder a toolbar contextual na viewport; preservar zonas de contexto/transformação/snap/pivô e visualização no topo, base para feedback da operação. Integrar ajuste da última operação à sessão modal existente, sem segunda fonte de estado. | Uma localização primária por ação/`CommandId`; picking/oclusão Object/Face/Edge/Point, orbit/pan/zoom/frame/ortho-perspectiva, quatro shadings canônicos + overlays, gizmo/modal preview/confirmar/cancelar/Undo agrupado, resize/HiDPI e paridade GPU/software. Idle segue render-on-demand. |
| 5 — PAINT, após estabilizar MODEL | Ordenar Brush/Color/Layers/Channels/Projection/Effects; trocar Canvas preto por preview de textura opcional; listas longas em ListView. | Brush/Color/Layers acessíveis e roláveis em qualquer altura suportada; sem inventar parâmetros ainda ausentes no motor. Fora da próxima discussão. |
| 6 — UV, após decisão canônica | Editor UV no centro em split 3D/2D redimensionável; Context só propriedades/ações; seleção sincronizada. | Split ~55/45, editor não é seção do Context, picking/transform/Undo e modo compacto testados. Bloqueado pela reconciliação da pill UV. |
| 7 — Tokens, escala e a11y | Consolidar medidas/estados em tokens, texto/ícone/comando por IDs, UI scale oficial, foco, tooltips e reduced-motion. | Matriz de estados default/hover/pressed/selected/focus/disabled; contraste e navegação F6/Tab/Shift+Tab/Space/Enter; sem novo hardcode público. |
| 8 — Polimento | Sombras, ícones e microinterações somente após as fases funcionais. | Regressões visuais aprovadas em temas Dark/High Contrast e escalas oficiais, sem queda relevante de performance/idle render-on-demand. |

### Foco imediato: MODEL e viewport

As fases 1–4 são o próximo recorte de trabalho. Primeiro estabilizar geometria
e rolagem do Context, porque reorganizar seções sem resolver o overflow apenas
desloca a falha. Em seguida, tratar a hierarquia do MODEL e a fronteira de
Tool Properties. Por último, migrar os comandos duplicados das barras sem
remover ações existentes. PAINT/UV não entram no próximo ciclo de implementação.

Para a fase 2, o owner proposto é `UiState.workspace_memory[Workspace]` com
chaves estáveis `SectionId` para o estado aberto/fechado. É memória de sessão,
sem serialização no documento; defaults dependem da seleção/tarefa. Testar a
troca de workspace durante criação de primitiva porque o fluxo atual pode
finalizar a operação pendente. Para a fase 3, a matriz de visibilidade é:

| Contexto MODEL | Seções primárias |
|---|---|
| Sem seleção | dica de próxima ação e estatísticas relevantes; sem Transform vazio |
| Objeto | Transform, Geometry/Material/Object Data pertinentes e Statistics |
| Face / Edge / Point | Selection primeiro, depois propriedades e ações daquele domínio; sem categorias de objeto irrelevantes |
| Referência | propriedades da referência, não campos de malha |
| Operação modal | parâmetros da ferramenta em região P3D-083, feedback e confirmação/cancelamento visíveis; Object Inspector não vira dono da operação |

## Gates de cada checkpoint

- Testes de estado e interação Slint, `cargo fmt --check`, `cargo check`,
  Clippy estrito, `arch-check`, `ui-guard --strict`, `docs-check` e `bible-check`.
- Capturas reais e comparação visual nas resoluções acima, com Dark/High
  Contrast e escalas oficiais (100/150/200% no gate mínimo); viewport WGPU e
  fallback software em Linux e Windows; medir mínimo da janela após
  estabilização do layout, não só no primeiro frame.
- Testar mouse, teclado, foco, scroll, resize, seleção por domínio, modal,
  Escape/Enter e Undo/Redo. Nenhum placeholder ou botão sem comando real.
- Atualizar matriz, plano de paridade, changelog e documentação de usuário
  afetada; não tocar no site público congelado. Bloqueios de gates são
  registrados explicitamente, nunca convertidos em aceite implícito.
- Auditar por símbolo visível a cobertura `TextId`, `IconId`, `ThemeToken` e
  `CommandId` antes/depois de cada migração; verificar árvore AccessKit,
  isolamento de painéis de plugin e ausência de regressão do estado de sessão.
