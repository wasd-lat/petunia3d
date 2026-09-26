# Seleção

O Petunia3D usa um **modelo de seleção unificado e contextual**: você escolhe o *domínio* da seleção — o objeto inteiro ou os componentes da malha (`Face`, `Edge`, `Point`) — e as ferramentas disponíveis se adaptam a ele. Não existe um "primeiro você precisa entrar no modo certo" obrigatório antes de começar.

> **Vocabulário:** na interface, pontos da malha aparecem como **Point**. Internamente (comandos, código e CHANGELOG) o mesmo conceito é `Vertex`, um detalhe técnico que você não precisa conhecer para modelar.

---

## 1. Domínios de seleção

| Domínio | O que seleciona | Texto de interface | Atalho | Comando |
| :--- | :--- | :--- | :--- | :--- |
| **Point** | Pontos tridimensionais individuais da malha. | `Point` | `1` | `select.domain_vertex` |
| **Edge** | Segmentos que conectam dois pontos da malha. | `Edge` | `2` | `select.domain_edge` |
| **Face** | Polígonos planos delimitados por arestas. | `Face` | `3` | `select.domain_face` |
| **Objeto** | Malhas inteiras na cena, como unidades atômicas. | `Object` | `4` | `select.domain_object` |

- **Alternar o domínio**: `select.cycle_domain` (`Tab` no preset Petunia) alterna de forma ágil entre a seleção de objeto (`Object`) e o último domínio de sub-elemento usado (`Point`, `Edge` ou `Face`). Pressionar `Tab` novamente retorna exatamente ao domínio de sub-elemento anterior.
- **Atalhos numéricos**: Os números `1`, `2` e `3` acessam diretamente os modos de componente, enquanto `4` acessa o modo de objeto. A tecla `0` não é mais utilizada para alternância de modo.
- **Selecionar o objeto antes dos componentes**: as ferramentas de malha operam sobre a **malha ativa**; selecione o objeto primeiro (ou clique direto na geometria, que ativa e seleciona).

### Seleção de objetos
- **Clique direto no viewport (`LMB`)**: o raio tridimensional é testado contra a cena e o objeto mais próximo e visível é ativado imediatamente.
- **Alternância com `Shift`**: adiciona ou remove objetos da seleção sem descartar os atuais.
- **Sincronização com o Outliner**: a seleção é bidirecional e reflete em tempo real na árvore de cena.

### Componentes da malha
Ao trabalhar com `Face`, `Edge` ou `Point`, os alvos de seleção ficam disponíveis na barra do viewport e as ferramentas contextuais de malha passam a operar sobre o que estiver selecionado.

---

## 2. Feedback visual

- **Pontos**: com o domínio `Point` ativo, os pontos da malha são demarcados por marcadores discretos.
- **Hover**: o elemento sob o cursor recebe realce antes do clique, para que você confirme visualmente o alvo.
- **Destaque de seleção**: elementos selecionados usam a cor de destaque do tema; a mesma semântica vale em todos os temas (dark, high contrast e temas do usuário).

Detalhes técnicos como a cor de hover pertencem aos `ThemeToken` — customizar tema não muda o comportamento de seleção.

---

## 3. Comandos de seleção

| Ação | Atalho (preset Petunia) | Comando |
| :--- | :--- | :--- |
| Selecionar tudo | `A` | `select.all` |
| Desmarcar tudo | `Alt+A` | `select.none` |
| Selecionar conectados | `Ctrl+L` | `select.linked` |
| Adicionar à seleção | `Shift` + clique | interação de ponteiro |
| Seleção por caixa (*Box Select*) | arrastar com `LMB` na área vazia do viewport | interação de ponteiro |
| Inverter seleção | `Ctrl+I` | `select.invert` |
| Alternar domínio | `Tab` | `select.cycle_domain` |

Todos os atalhos acima pertencem ao **preset de keymap ativo**. Se você usar os presets `Blender-like`, `Maya-like`, `3ds Max-like` ou `Cinema 4D-like`, os binds mudam — consulte [Atalhos](../shortcuts/) e [Keymaps](../input/keymaps.md). Nenhuma ferramenta depende de uma tecla física específica como regra.
