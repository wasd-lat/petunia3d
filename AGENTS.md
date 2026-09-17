# AGENTS.md — Regras obrigatórias para todos os code agents

> Leia este arquivo **antes** de qualquer tarefa neste repositório. Ele é normativo.
> Em conflito com qualquer outra instrução do repositório, este arquivo prevalece;
> em conflito com o caderno canônico, o caderno prevalece sobre este arquivo.

## 0. Fonte única da verdade

O caderno canônico vive em **`docs/bible/`** — 249 páginas: raiz `index.md`
(Petunia3D — Livro Vivo), `constitution/` (00–16), `foundations/` (01–44),
`specs/` (P3D-001 a P3D-168), `sections/` (A–O), `addenda/` e `status/`.

- Toda decisão de arquitetura, produto, UX, escopo, vocabulário e documentação
  deve sair do caderno. Nenhuma outra documentação pode contradizê-lo.
- A antiga **Implementation Bible** foi absorvida pelo Livro Vivo. Referências a
  esse nome são alias histórico — nunca uma segunda autoridade.
- O espelho em `docs/bible/` é **derivado e verificado**. Não edite uma página
  do caderno em dois lugares: o caderno é o único lugar.
- Hierarquia de autoridade em conflito (cap. 13): `32` (ADR Odin→Rust) → `34`
  (representação/ownership/Rust safety) → `27–31`, `35`, `36` (UI Baseline final)
  → `09` → `21` → `12` → `14–20` → `22–26` → capítulos especializados → `07`
  (pesquisa, não requisito). Contradição não resolvida deve ser **sinalizada**,
  nunca escolhida em silêncio.

## 0.1 Diretiva de intervenção vigente (temporariamente normativa)

Enquanto a iniciativa de UI estiver em andamento, vale também:

**`PETUNIA3D_EGUI_ECOSYSTEM_FINAL_PUSH_DIRECTIVE.md`** — *Egui Ecosystem Final
Push v2*. Por decisão explícita do próprio documento (§0), ele é a **diretiva de
intervenção** para code agents:

1. identifica conflitos concretos entre código e documentação;
2. define a direção a aplicar para resolvê-los;
3. ordena a atualização das fontes documentais canônicas;
4. impede que um agente continue seguindo regras antigas incompatíveis.

Consequências operacionais:

- a diretiva **vence** qualquer página que ainda diga o contrário — em especial a
  política antiga de "built-in primeiro", "Taffy somente piloto" e
  "`egui_tiles` removido";
- ela **não** substitui permanentemente o caderno: ao fim das waves, o caderno
  incorpora as decisões e a diretiva vai para histórico/auditoria;
- **nenhum agente** pode interpretar "preservar a stack egui" como "preservar a
  arquitetura de layout atual".

Regra de layout derivada (§17, §36):

> Raw egui para composição simples. Bibliotecas especializadas para problemas
especializados. Petunia Components e Adapters como única superfície permitida
para product UI.

Proibido em product code (fora de foundation/adapter e das exceções de §36 —
viewport, canvas UV, régua da timeline, gizmo, visualização de dados,
implementação de componente de baixo nível):

- `ui.available_width() < N` como responsividade;
- divisão manual de largura;
- `ui.spacing_mut()` fora de foundation;
- `allocate_exact_size` / `painter().rect_filled` / `add_space` quando já existe
  componente padrão;
- tipos de `egui_taffy`, `egui_tiles`, `egui_dnd`, `egui_animation`, `egui_form`
  ou `twill` fora do adapter;
- tempo de animação em milissegundos nas APIs do egui — use
  `foundation::motion::seconds(..)` (a unidade é **segundo**; ms deixa a
  transição ~1000× mais lenta sem quebrar nada).

Contratos que já substituíram fórmulas manuais (não os reimplemente):

- colunas de largura igual → `columns` + `PetuniaColumnSpec` (§45);
- largura que cede ao espaço → `clamped_width` / `fill_remaining` (§45);
- barra com overflow → `PetuniaResponsiveToolbar` (`adapters::toolbar`, §46);
- header de três zonas com centro geométrico → `PetuniaTopBar`
  (`adapters::top_bar`, §25) — **não** use `ui.columns(3, ..)`: colunas iguais
  põem o centro no meio do terço, não no meio da barra;
- macro-layout do shell → `PetuniaLayoutAdapter` + `PetuniaShellLayout`
  (`adapters::tile_layout`, §31); o contrato durável é o DTO Petunia, nunca a
  árvore da crate;
- grade de itens com colunas derivadas (paleta de ferramentas) →
  `PetuniaToolGridSpec` (`adapters::tool_grid`, §48): declare `min_cell` e
  `label_min_cell`; o adapter devolve `PetuniaToolCell { width, columns, labeled }`
  e **nunca** escreva `if available_width() >= N` para decidir arranjo;
- campo rotulado (rótulo + controle) → `PetuniaForm` (`adapters::form`, §48):
  `field`, `toggle`, `section`; a decisão `Inline`/`Stacked` sai de `plan_row`
  (largura do rótulo × mínimo do controle), não de um breakpoint;
- validação por campo → `PetuniaValidationReport` + `PetuniaFormSession`
  (`adapters::form`, §51): o **domínio** monta o relatório (ex.:
  `Keybinds::detect_conflicts`), o contrato mostra (`validated_control`,
  `error_summary_titled`, `reveal_errors`). Nada de banner com cor literal;
- lista reordenável → `PetuniaDragList` (`adapters::drag_drop`, §49): o adapter
  desenha o esqueleto da linha (grip + conteúdo) e aplica a ordem no drop; o
  produto só desenha o conteúdo. Setas ↑/↓ só existem onde o arrasto não chegou;
- animação de estado → `PetuniaMotion` (`foundation::motion`, §49): `reveal`,
  `animate`, `position`, `section`. Feedback de estado, nunca decoração contínua;
- item horizontal dentro de um layout taffy →
  `PetuniaResponsiveLayout::with_item_layout(PetuniaItemLayout::Row)` — sem isso o
  item herda o layout vertical do painel e empilha os próprios filhos;
- largura de um controle dentro de uma célula mede-se pelo `cell`/largura que o
  adapter entregou (`PetuniaToolbarButton::width`), nunca pelo container.

O callback de item de uma grade taffy roda **mais de uma vez por item** (medida +
desenho): é um desenho, não um acumulador — não acumule estado nele.

Largura mínima que um painel precisa para hospedar um controle é **derivada** do
controle (ver `tokens::TOOLBAR_MIN_WIDTH` = ícone + vão + seta + moldura), não
herdada de política antiga de painel. Controle desenhado fora do paine é invisível
e inalcançável — `crates/ui/tests/toolbar_fit.rs` guarda o invariante para a
paleta.

Guard: `cargo run -p xtask -- ui-guard` (relatório) e `--strict` (falha se um tipo
auxiliar escapar do adapter). Baseline congelada em
`docs/audits/ui-ecosystem-final-push/00-baseline.md` e o estado atual das waves em
`docs/audits/ui-ecosystem-final-push/` (`01`…`05`).

O `docs-check` inclui a validação do mapa de componentes (`docs/public/ui-map.json`)
contra o código. Esse arquivo está no **site público congelado** (AGENTS.md §1):
quando o código muda o símbolo de entrada de um nó, o mapa precisa de uma decisão
explícita de descongelar (`cargo xtask bible-lock`) — não edite o arquivo por
conta própria.

Ordem obrigatória para componente reutilizável (§38):

> 1. procurar componente existente; 2. modificar o Component Gallery; 3. só
> depois usar no produto.

```bash
cargo run -p petunia_ui --example component_gallery
```

Não criar visual isolado dentro de um painel quando ele é reutilizável. O que
ainda não existe como componente fica registrado na própria gallery como linha
`pendente · <nome>` com o contrato que falta (`PENDING` em
`crates/ui/src/gallery.rs`) — a gallery não pode parecer completa quando não é.

## 1. 🧊 SITE DE DOCUMENTAÇÃO CONGELADO

**Decisão de 2026-09-16: o site público de documentação está congelado até o fim
do desenvolvimento de todo o projeto.** Nenhum agente deve trabalhar nele.

Congelado (não modificar, não "melhorar", não corrigir estilo):

- `docs/.vitepress/**` — config, nav, `bibleSidebar.ts`, tema, cache, `dist/`;
- `docs/index.md` — página inicial/hero do site;
- `docs/public/**`, `docs/package.json`, `docs/pnpm-lock.yaml`, `docs/vercel.json`;
- `.github/workflows/docs.yml` — publicação/deploy do site;
- `docs/image-references/**` — mockups e capturas usados como referência visual;
- promessa de site: busca, versionamento, i18n do chrome, screenshots publicados.

Motivo: o caderno é a verdade; o site é superfície de apresentação. Manter o site
enquanto o produto muda gera retrabalho e divergência.

Uma exceção **não** congelada é o *conteúdo* de documentação que agentes e
contribuidores leem (tudo fora dos caminhos acima): se ele contradiz o caderno,
corrija o conteúdo. Reconstruir o site a partir do caderno é trabalho de fim de
projeto.

## 2. Regra de reconciliação antes de implementar

1. Auditar o código e o comportamento executável antes de mudar qualquer coisa.
2. Classificar cada requisito relevante como `COMPLIANT`, `PARTIALLY_COMPLIANT`,
   `FUNCTIONAL_BUT_DIFFERENT`, `RUDIMENTARY`, `STUB`, `BROKEN`, `DUPLICATED`,
   `MISSING` ou `OBSOLETE` — produzir uma **Implementation-vs-Spec Gap Matrix**.
3. Preservar o que já é `COMPLIANT`. Corrigir apenas o delta comprovado.
4. Reescrita completa só com evidência de que a arquitetura atual impede a
   correção incremental. Nada de big-bang rewrite.

## 3. Invariantes que nunca podem ser violadas

- Core e domínio não dependem de `egui`, `eframe`, widgets, cores, ícones ou teclas.
- Strings visíveis usam `TextId`; ícones usam `IconId`; aparência usa `ThemeToken`;
  ações semânticas usam `CommandId`. **Zero hardcode** de texto, ícone, cor ou
  atalho físico em UI pública.
- Input físico é resolvido por keymap. Tools não conhecem teclas como regra de negócio.
- Cadeia funcional: `Tool → Command → Algorithm → Data`. Tool não chama Tool.
- Mutations são transacionais; documento com single-writer; jobs operam em snapshots.
- `unsafe` isolado e auditável.
- UI Baseline V1 congelada (cap. 36): workspaces `MODEL / PAINT / UV`, shell
  Parts-esquerda / Context-direita / Asset Library-abaixo, dark oficial,
  Petunia Components como linguagem visual. Não reintroduzir docking irrestrito,
  clone de Blender, acesso cru a egui/wgpu para plugins nem reabrir `UI-OPEN`.
- Vocabulário de usuário (cap. 13): **Point**, **Round Edge**, **Fuse**, **Cut**,
  **Connect**, **Keep Parts**, **Join**, **Project From Reference/View**.
  `Vertex`, `Bevel`, `Union`, `Difference` são termos técnicos.

## 4. Definition of Done

Uma feature só termina quando, conforme aplicável: comportamento, testes,
arquitetura, UI/tokens, documentação, screenshots, changelog e referências
geradas estiverem sincronizados — e **nenhuma documentação conhecida como
obsoleta permanecer**. Gates obrigatórios: `cargo fmt --check`, `cargo check`,
testes relevantes, `cargo clippy` quando viável, architecture checks,
`cargo run -p xtask -- docs-check`, `cargo run -p xtask -- bible-check` e
`cargo run -p xtask -- ui-guard --strict` (confinamento de tipos de UI).

## 5. Protocolo de contexto

Leia, nesta ordem: este `AGENTS.md` → `ENTRYPOINT.md` → `prumo.json` →
`docs/PRUMO.md` → a página relevante em `docs/bible/` → o código e os testes
envolvidos. Contexto mínimo suficiente, expansão progressiva, ponteiro em vez de
payload. Nunca enfraquecer critérios de aceitação em silêncio.

## 6. Política de modelos (assinatura OpenCode Go) — obrigatória

Cada função tem **um único modelo autorizado**. Os agentes de função vivem em
`.opencode/agent/` com o modelo fixado no frontmatter (`model:`); esta tabela
é a autoridade quando houver divergência.

| Função | Modelo exigido | Agente |
|---|---|---|
| Matemática/engenharia (algoritmos, malha, UV math, WGSL, compositing, graph) | `opencode-go/deepseek-v4-pro` | `math-core` |
| UI/UX (painéis, tokens, componentes, layout, i18n de interface) | `opencode-go/grok-4.6` or `openai/gpt-5.6-luna`| `ui-ux` |
| Review visual (screenshots, regressão visual) | `opencode-go/minimax-m3` | `vision` |
| Execução mecânica (testes, migrações, scaffolding, gates locais) | `opencode-go/deepseek-v4-flash` | `worker` |
| Revisão crítica (correctness, segurança, arquitetura, vereditos) | `opencode-go/gpt-5.6-luna` | `reviewer` |
| Documentação e textos (caderno, specs, changelog, i18n) | `opencode-go/qwen3.7-plus` | `docs` |

IDs conforme o catálogo Go vigente — confirmar com `/models` no TUI; o catálogo
rotaciona. Se um ID não resolver, **atualizar esta tabela e o frontmatter do
agente antes de prosseguir** — nunca improvisar outro modelo em silêncio.

**MODEL GATE — vale para todo agente, inclusive sessão primária:**

1. No primeiro turno, identifique sua função pela tarefa e seu modelo atual
   pelo contexto da sessão.
2. Se o modelo atual **não** for o exigido para a função: **PARE**. Não execute
   nenhuma ferramenta de trabalho. Avise o usuário (modelo atual × modelo
   exigido) e **só continue após confirmação de que o modelo foi alterado**.
3. Orçamento: limites em dólar ($12/5h, $30/semana, $60/mês). Pro só onde o
   raciocínio compensa (Fases 1–3 do plano Paint); volume vai para Flash.
