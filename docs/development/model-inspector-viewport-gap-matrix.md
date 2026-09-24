# MODEL Inspector / viewport — Implementation-vs-Spec Gap Matrix (rodada 2)

Escopo: pedido de 23/09/2026, 11 pontos. Auditoria inicial da UI Slint de produção, bridge, Core e renderers. Autoridade: Livro Vivo 05, 23, 24, 36 e decisão explícita do usuário de revisar a baseline para Parts como primeira seção do Inspector direito. O shell continua viewport-first e sem docking irrestrito.

| # | Requisito | Estado inicial | Evidência / delta necessário |
| --- | --- | --- | --- |
| 1 | Parts dentro do Inspector direito; Parts, Transform, Material, Object colapsáveis, responsivos | BROKEN | Parts é drawer separado que sobrepõe o trilho esquerdo; Context não rola, ordem atual é Boolean/Transform/Object/Material e ações soltas. Revisar baseline/ADR e migrar sem perder o acesso em janela compacta. |
| 2 | Transform geral + parâmetros de primitiva; Object info discreta | RUDIMENTARY | Vector3Field transacional existe; `PrimitiveDescriptor` existe somente durante a criação e não persiste no Asset, logo raio/lados não podem virar controles pós-criação sem novo contrato de dados/Undo. Object info ocupa seção grande. |
| 3 | Select único com click/box-drag; Lasso | PARTIALLY_COMPLIANT | `pending-box-select` já dá click/box com Select; botão Box Select duplicado continua na shelf. Não há lasso nem seleção por polígono no Core. |
| 4 | Move, Rotate, Scale e gizmo combinado; ícones claros | PARTIALLY_COMPLIANT | Três ferramentas e gizmos operam; não há gizmo combinado nem iconografia distinta suficiente. |
| 5 | Quatro ícones circulares de shading distinguíveis | FUNCTIONAL_BUT_DIFFERENT | Quatro SVGs novos usam metáfora de cubo; dois modos expostos (`Material`/`Rendered`) divergem dos modos canônicos `Textured`/`Silhouette` e precisam de implementação/ADR, não só renomeação. |
| 6 | Nome + descrição em tooltip de ferramentas, ícones e campos | MISSING | Há accessible-label, mas nenhum Tooltip na UI Slint; usar componente nativo compartilhado, traduções `TextId` e foco de teclado. |
| 7 | Point/Edge legíveis; sem dots poluidores em Face | PARTIALLY_COMPLIANT | Discos Point e faixas Edge existem em GPU/software, mas contraste/tamanho/hit ainda exigem QA; `compute_asset_overlay` desenha dots em todos os centros de face. |
| 8 | Wire overlay independente do modo Wireframe, combinável com X-Ray | BROKEN | Estado `show_wireframe_overlay` já existe e renderers o respeitam, mas botão do popover chama `view.toggle_wireframe`, que alterna o modo-base, não o overlay. |
| 9 | Movimento por atalho previsível como modal do Blender | PARTIALLY_COMPLIANT | G/R/S iniciam sessão modal e mouse move sem clique, mas falta teste com ponteiro inicial longe do pivô, zoom/ortográfica/perspectiva, movimento contínuo, confirmação/cancelamento e mudança de restrição sem salto. |
| 10 | Silhueta/overlay de esfera corretos | BROKEN | Contorno Object usa adjacência front/back com `camera.forward` constante e projeta segmentos sem clipping/oclusão; verificar esfera triangulada e corrigir o algoritmo. |
| 11 | Marca/menus alinhados à esquerda; menu fecha ao click-away | BROKEN | `HorizontalLayout` do chrome não preenche explicitamente a largura; callback `click-away-requested` fecha a pilha, mas não sincroniza `menu-open` ao shell. |

Esta matriz será atualizada por evidência de código, teste headless, captura nativa e gates. Nenhum item vira `COMPLIANT` apenas porque um controle foi desenhado.

## Matriz de ferramentas MODEL/viewport — rodada de interação

Escopo solicitado: Loop Cut com descoberta e preview por hover, wheel sem
interferência de navegação, Push/Pull, Slice, Profile e selector de pivot.
Referências: P3D-027, P3D-029, P3D-034, P3D-083, P3D-131; Livro Vivo 02, 03 e
14. Comportamentos Blender foram conferidos no manual 5.2 para Loop Cut/Slide,
Extrude e Pivot Point. A meta é reaproveitar o modelo mental de preview,
confirmação e cancelamento, respeitando a semântica própria do Petunia.

| Requisito | Estado antes | Evidência e delta | Estado após implementação |
| --- | --- | --- | --- |
| Loop Cut pode iniciar pela ferramenta sem seleção prévia de aresta | BROKEN | `begin_loop_cut()` exigia `selected_edges`; `LoopRing::discover()` já oferece a validação correta. Tool armada passa a descobrir anel pela aresta visível sob o cursor. | PARTIALLY_COMPLIANT — descoberta/seleção apenas em edge quad elegível; falta QA visual nativo com vários tipos de topologia. |
| Preview do corte acompanha hover e não altera documento | BROKEN | `LoopRing::preview()` existia, mas não era projetado pelo VM; mesh só mudava depois do início da sessão. Agora o shell projeta segmentos para overlay e hover não registra Undo nem altera malha. | PARTIALLY_COMPLIANT — coberto por teste do bridge; comparação visual nativa ainda pendente. |
| Wheel ajusta a quantidade de loops | PARTIALLY_COMPLIANT | Scroll já mudava cuts durante sessão; agora também muda cuts durante o preview hover. O limite continua 1–32 e o mesmo evento não chega ao zoom. | PARTIALLY_COMPLIANT — teste de integração Slint valida callback Cuts e ausência de callback Zoom; falta reprodução com dispositivo físico/trackpad. |
| Navegação não rouba gestures durante ferramentas modais/interativas | BROKEN | Handler Slint sempre emitia Orbit/Pan e Zoom; callbacks não filtravam tool ativa. Navegação fica suspensa em Loop Cut, Slice, Profile e modal de ferramenta. | PARTIALLY_COMPLIANT — guard bridge + teste headless de wheel; orbit/pan e touchpad ainda pedem QA de hardware. |
| Loop Cut posiciona, permite slide e confirma/cancela em uma transação | PARTIALLY_COMPLIANT | A sessão de preview/slide/undo já existia, mas dependia de aresta previamente selecionada. Click no edge hover cria sessão; Enter confirma e Esc restaura. | PARTIALLY_COMPLIANT — teste cobre hover→count→place→commit único; falta teste do gesto de slide nativo no novo início por hover. |
| Push/Pull está visível e trabalha com preview transacional | PARTIALLY_COMPLIANT | `ModalKind::PushPull`, kernel e modal parametrizado já existiam; a shelf Slint não expunha a ação. Foi adicionado botão direto e dica localizada. | PARTIALLY_COMPLIANT — integração reutiliza modal existente; testes de kernel/modal já existem, falta teste de gesto do controle recém-exposto. |
| Slice mantém os dois lados por padrão | BROKEN | `CutSession::compute_slice()` chamava `slice_plane(..., true)`, descartando um lado. Agora usa `false`, preservando ambos e sem cap por padrão conforme Livro Vivo 14. | PARTIALLY_COMPLIANT — teste confirma divisão topológica nos dois lados; preview visual atual ainda é guia 2D do plano, não renderização da mesh resultante. |
| Slice bloqueia navegação e cancela sem commit | PARTIALLY_COMPLIANT | Já existia sessão e confirmação no release; Slice agora desativa orbit/pan/zoom enquanto está armada ou arrastada. | PARTIALLY_COMPLIANT — session cancel existente; falta gesture regression com Esc durante arrasto e QA visual do plano. |
| Profile permite desenho, fechamento, preview e saída por extrude/revolve | RUDIMENTARY | Estado e algoritmos existiam no módulo, mas o shell de produção Slint não ativava a captura nem fornecia fluxo de gerar. Adicionados seleção de ferramenta, preview da linha, fechamento explícito, profundidade e ações Generate/Revolve. Extrude e Revolve respeitam frame right/up/normal do Profile. | PARTIALLY_COMPLIANT — teste headless fecha perfil e gera mesh com um Undo; falta precisão do plano sob vistas ortográficas/perspectiva e interação de cancelamento visual. |
| Pivot tem selector acessível no viewport e política atual chega ao transform core | RUDIMENTARY | Enum, cálculo e uso pelo modal já existem no domínio; Slint não mostrava selector. Adicionado popover localizado para Median, Bounds, Cursor e Individual Origins. | PARTIALLY_COMPLIANT — teste valida escolha Cursor e VM; Active Element permanece fora da enum/spec aprovada e casos de seleção múltipla ainda precisam auditoria P3D-027. |

### Fontes Blender consultadas

- [Loop Cut](https://docs.blender.org/manual/en/latest/modeling/meshes/tools/loop.html)
- [Loop Cut and Slide](https://docs.blender.org/manual/en/latest/modeling/meshes/editing/edge/loopcut_slide.html)
- [Extrude Faces](https://docs.blender.org/manual/en/latest/modeling/meshes/editing/face/extrude_faces.html)
- [Transform Pivot Point](https://docs.blender.org/manual/en/latest/editors/3dview/controls/pivot_point/index.html)

No Blender, o Loop Cut parte de uma aresta sob o cursor em malha elegível, exibe
uma prévia do edge ring, usa a roda para aumentar/reduzir cortes e mantém uma
etapa modal para posicionar/slide antes da confirmação. O Petunia reaproveita
esse ciclo, mas limita a descoberta a quad rings válidos e informa falha para
triângulo/ngon/non-manifold em vez de adivinhar o caminho. Profile continua
sendo uma ferramenta shape-first do Petunia, não uma cópia de uma tool Blender.

## Reconciliação desta rodada

| # | Estado após a implementação | Evidência e limite ainda aberto |
| --- | --- | --- |
| 1 | PARTIALLY_COMPLIANT | Parts foi integrado ao Inspector direito; seções usam altura de conteúdo, scroll e recolhimento independente. Busca/filtro/ordenação/contagem compartilham uma faixa; altura da linha reage ao slider no Inspector e no drawer compacto. Teste headless verifica alinhamento, acessibilidade e resizing nos dois layouts. Capturas nativas em 1034×1012 e 850×700 confirmaram a ordem, Alt+clique e acesso pelo drawer compacto. Faltam lista longa e persistência do estado entre sessões; ainda falta aceite visual nativo para a nova separação de seções. |
| 2 | PARTIALLY_COMPLIANT | Object ficou compacto, com Duplicate/Delete dentro da seção. Parâmetros paramétricos da primitiva continuam ausentes: `PrimitiveDescriptor` não é persistido no Asset; requer contrato de proveniência, regeneração transacional e Undo. |
| 3 | PARTIALLY_COMPLIANT | Select mantém clique/caixa; botão de Box redundante foi removido e Lasso adicionado. Caixa/laço agora detectam arestas que cruzam a região, com testes geométricos. Ainda faltam seleção de região inteiramente dentro de uma face, cobertura de oclusão e teste E2E do gesto. |
| 4 | PARTIALLY_COMPLIANT | Quatro ferramentas e handles distintos do gizmo combinado foram adicionados; teste de picking de move/scale/rotate passou. Falta QA de precisão do gesto e dos estados de foco/hover. |
| 5 | PARTIALLY_COMPLIANT | SVGs circulares distintos foram adicionados aos quatro modos expostos. Nomes/semântica de Material/Rendered versus os modos canônicos ainda exigem reconciliação de produto. |
| 6 | PARTIALLY_COMPLIANT | Componentes reutilizáveis de ação, ferramenta, campo e seção receberam Tooltip; rótulos principais de MODEL foram traduzidos via `TextId`. Controles diretos antigos e descrições de algumas seções ainda precisam de cobertura. |
| 7 | PARTIALLY_COMPLIANT | Dots de Face removidos; Points/Edges ganharam presença visual nos dois renderers. Hit target e legibilidade sob DPI alto/cores customizadas ainda precisam de QA nativo. |
| 8 | COMPLIANT | `view.toggle_wire_overlay` alterna somente o overlay e preserva o modo-base; teste de independência com X-Ray e shading passou. |
| 9 | PARTIALLY_COMPLIANT | Modal por atalho aceita movimento no mesmo gesto; clique de confirmação não vaza para seleção. Teste de gesto cobre o fluxo básico; faltam matriz completa de câmera/restrição e QA nativo. |
| 10 | PARTIALLY_COMPLIANT | Silhueta de esfera usa orientação local de cada face e rejeita projeção inválida; teste de finitude/limites passou. Falta comparação visual em múltiplas câmeras e X-Ray. |
| 11 | PARTIALLY_COMPLIANT | Marca/menus foram alinhados no chrome; click-away sincroniza o estado de menu. Testes de estado passaram; falta QA de monitor pequeno e navegação por teclado. |

`COMPLIANT` acima significa somente o delta específico verificado, não paridade global com Blender/Cinema 4D/Plasticity/Blockbench. O checkpoint continua incremental e as pendências não foram reclassificadas como prontas.
