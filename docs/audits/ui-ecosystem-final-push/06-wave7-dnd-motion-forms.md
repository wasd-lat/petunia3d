# 06 — Wave 7 · DnD, motion e validação de formulário (§49, §51)

<aside>
🧭

Escopo da wave: **adoção real** de `egui_dnd`, `egui_animation` e `egui_form` —
primeiro em um path de produto cada, não em tudo de uma vez. Três armadilhas de
crate foram medidas e estão no ledger; uma delas (a unidade do tempo de animação)
é do tipo que falha em silêncio.

</aside>

## Resumo

| Entrega | Antes | Depois |
| --- | --- | --- |
| Reordenar a paleta | setas ↑/↓ (`chevron_toggle_dir`) | [`PetuniaDragList`] — grip arrasta, ordem aplicada no drop |
| Motion | só durações em `foundation::motion` | backend real: [`PetuniaMotion::reveal`], `animate`, `position`, `section` |
| Revelar busca (Outliner e Inspector) | a linha aparecia de uma vez | altura interpolada pelo motion do sistema |
| Validação em Settings | banner com cor literal na tela | [`PetuniaFormSession`] + `error_summary_titled` + marca por campo |
| Cor de erro | `from_rgb(239, 83, 80)` na tela | `tokens::ACCENT_ERROR` / `ACCENT_ERROR_SOFT` |
| `available_width` em product code | 14 | **14** (sem regressão) |
| `spacing_mut` em product code | 37 | **37** (sem regressão) |

```
✅ cargo fmt --check · clippy 0 warnings (workspace, --all-targets)
✅ 725 testes passando, 0 falhando (270 na lib da UI)
✅ ui-guard --strict exit 0 · bible-check · arch-check · docs-generate --check
⚠️ docs-check: falha apenas no passo `ui-map` (mapa congelado — ver Pendências da wave 5/6)
```

---

## 1. DnD: o grip precisa ser maior que o primeiro movimento

O motor (`egui_dnd` 0.17) só sai de "esperando limiar de clique" para "pode
arrastar" quando o ponteiro **já andou mais de 1px e ainda está sobre o handle**
— ou quando passam 250ms de pressão contínua. Medido com sonda: um primeiro passo
de 6px para fora de um grip de 18×20px **não** inicia o arrasto; o mesmo passo
começando dentro do grip inicia.

Isso virou invariante de projeto, não detalhe: o grip não é decorativo, ele é a
área de aquisição do gesto (`PetuniaDragSpec::grip_width/height`). O teste
`a_real_drag_reorders_the_list_on_drop` caminha dentro do grip no primeiro passo
justamente porque é assim que um dedo/mouse reais começam.

**Segunda armadilha, também medida:** o `to` que o motor informa é *limite
exclusivo* quando o arrasto desce (`to = 2` partindo de `0` deixa o item na
posição **1**). [`apply_drag_update`] converte para a semântica de inserção do
Petunia, e
`the_adapter_matches_the_engine_order_for_every_pair` verifica a equivalência
contra `egui_dnd::utils::shift_vec` **para todos os pares** de posições em uma
lista de 5 itens.

**Terceira:** o closure da linha é um desenho, não um acumulador — durante o
arrasto a crate o chama de novo para o item flutuante (`n + 1` chamadas por
frame, medido).

### O que saiu

```rust
// antes: duas setas + swap manual depois do laço
if crate::widgets::chevron_toggle_dir(ui, &down_tip, ChevronDir::Down).clicked() { move_down = Some(idx) }
...
order.swap(idx, idx + 1); state.ui.toolbar_order = order.clone();

// depois: declarar a lista; o adapter desenha o esqueleto da linha
TOOLBAR_ORDER_DRAG.show(ui, "toolbar_config_order", &mut order, |id| Id::new(("toolbar-order", id)), |ui, id, row| {
    // checkbox de visibilidade + rótulo; `row.dragging` colore a linha
});
```

Escritor único: [`apply_toolbar_config`] grava ordem e visibilidade no estado e
responde se houve mudança — comparar antes de escrever evita `mark_dirty` por
frame (`the_drag_order_and_the_visibility_marks_land_in_the_state`).

---

## 2. Motion: a unidade do tempo é o erro fácil

`egui_animation` (e o próprio egui) medem tempo em **segundos**. Passar
`millis(BASE)` = `160.0` deixa a transição ~1000× mais lenta: medido em sonda,
**0,1% por frame** em vez dos 160ms declarados. Nada quebra — só parece que não
anima.

Por isso `foundation::motion` ganhou `seconds()` (conversão explícita, `const fn`)
e um teste que *falha* se a unidade voltar a ser errada:
`a_reveal_advances_at_the_declared_rate` exige >0,9 após 8 frames de 20ms com
`BASE` (160ms).

Contrato mantido (capítulo 36): nenhuma animação contínua decorativa. As quatro
funções existem para **estado**:

| função | caso |
| --- | --- |
| `reveal` | alpha 0↔1 de algo que aparece/some |
| `animate` | valor numérico interpolado |
| `position` | item que encaixa (mesma máquina do DnD) |
| `section` | seção que abre/fecha animando a altura |

Com `style.animation_time == 0` (acessibilidade/preferência) todas viram troca
instantânea — testado (`reveal_falls_back_to_a_step_when_motion_is_off`,
`a_closed_section_with_motion_off_draws_nothing`).

**Path real escolhido:** a revelação da busca inline do Outliner e do Inspector —
exatamente o "revelar campo" que a duração `BASE` descreve. Nada de animar o shell
inteiro de uma vez.

---

## 3. Validação: o domínio continua dono da regra

`egui_form` entrou como **backend de validação** do mesmo `PetuniaForm` da Wave 6
— Settings não foi reescrito. O arranjo continua sendo do `egui_taffy` (rótulo
declarado + controle com o resto); a crate acrescenta erro por campo, estado de
revelação e foco no primeiro inválido.

```text
Keybinds::detect_conflicts()        (domínio — já existia)
        ↓
PetuniaValidationReport             (dado: campo → mensagem)
        ↓
PetuniaFormSession                  (um frame; a crate fica dentro)
        ↓
validated_control · error_summary_titled · reveal_errors
```

O primeiro path é o **aba de Keymap**: cada conflito vira erro do campo
`action_a` com o atalho e o outro dono na mensagem; o resumo inline substituiu o
banner desenhado à mão (que tinha três cores literais — agora
`tokens::ACCENT_ERROR`/`ACCENT_ERROR_SOFT`).

**Gesto de confirmar:** escolher um perfil de teclado pode trazer conflitos; é
nesse clique que `reveal_errors` marca **todos** os campos inválidos de uma vez e
foca o primeiro (`revealing_errors_focuses_the_invalid_field`). Antes disso o
usuário vê o resumo — o erro por campo não pisca enquanto ele digita.

---

## 4. O que **não** foi feito (de propósito)

- **Reordenar a lista de modificadores por arrasto.** O contrato existe e funciona;
  trocar as setas do Modifier Stack é a próxima expansão da §49 — declarado na
  vitrine, não escondido. Uma wave, um path.
- **`egui_table`, `egui_virtual_list`, `egui_suspense`.** Seguem fora do grafo
  (Wave 8/9): instalar sem path real seria adoção decorativa.
- **Tabelas do Context/Asset Library com `egui_table`.** Wave 8/9.
- **Captura de screenshots** (§40/§42) — continua no passe final, como decidido.

## 5. Achados

| # | Achado | Evidência |
| --- | --- | --- |
| A14 | `egui_dnd` só inicia o arrasto enquanto o ponteiro está sobre o handle (ou após 250ms) | sonda + `a_real_drag_reorders_the_list_on_drop` |
| A15 | `to` do `egui_dnd` é limite exclusivo quando o arrasto desce | `the_adapter_matches_the_engine_order_for_every_pair` |
| A16 | `egui_animation`/egui medem animação em segundos; ms deixa ~1000× mais lento | `a_reveal_advances_at_the_declared_rate` |
| A17 | o closure de linha do `egui_dnd` roda `n + 1` vezes durante o arrasto | sonda |
| A18 | `egui_form` mostra erro por campo **só** depois de revelado, e o reveal funciona para widgets sem foco (o `try_submit` marca a memória) | `revealing_errors_focuses_the_invalid_field` |
| A19 | a revelação precisa de um frame anterior no valor oposto: sem ele a primeira animação nasce em 1.0 | `reveal_ramps_up_then_settles_and_ramps_down` |

## 6. Arquivos tocados

| Arquivo | Papel |
| --- | --- |
| `crates/ui/src/adapters/drag_drop.rs` | **novo** — contrato de lista reordenável |
| `crates/ui/src/foundation/motion.rs` | backend real de animação + `seconds()` |
| `crates/ui/src/adapters/form.rs` | relatório, sessão, campo validado, resumo |
| `crates/ui/src/toolbar.rs` | config da paleta por arrasto; chevrons removidos |
| `crates/ui/src/outliner.rs` · `properties_panel.rs` | busca revela animando |
| `crates/ui/src/settings_modal.rs` | validação de keymap pelo contrato |
| `crates/ui/src/tokens.rs` | `ACCENT_ERROR`, `ACCENT_ERROR_SOFT` |
| `crates/ui/src/gallery.rs` | vitrines de lista reordenável, motion e form validado |
| `assets/locales/{en,pt-BR}.toml` | `keymap.conflict_title`, `keymap.conflict_shared` |
