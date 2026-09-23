# Implementação versus especificação — viewport

Base: `f94b888`. Atualização: 23/09/2026. Fontes: Livro Vivo local,
`Markdown(2).md colado` e três capturas anexadas pelo usuário.

Os checkpoints de 22/09 eram não testados. Nesta rodada, os testes de domínio
Paint (22), UV (4) e shell Slint (169) passaram com Rust 1.98.1. Isso comprova
os fluxos exercitados em código, mas ainda não substitui captura visual nem
reprodução manual nos dois backends.

| Fluxo | Base | Causa observada | Implementação neste ramo | Aceite |
|---|---|---|---|---|
| Contorno, marcadores e gizmos | BROKEN | Slint Path usa ajuste ao bounding box em coordenadas já projetadas | `ImageFit.preserve`; tripé em coordenadas locais e clipping | Pendente |
| Troca de seleção | BROKEN | Frame não redesenhado e flags de domínio residual | Conversão de seleção, limpeza de hover e redraw no callback | Pendente |
| Picking | PARTIALLY_COMPLIANT | Bounding sphere e tolerância em mundo | Raycast de faces; alvos em pixels; hover/clique usam mesmo picker | Pendente |
| Oclusão | PARTIALLY_COMPLIANT | Raio do cursor usado para candidato deslocado | Consulta por lote da cena e raio pelo candidato em perspectiva/ortográfica | Pendente |
| Polígonos côncavos | BROKEN | Fan cobre regiões exteriores do polígono | Triangulação compartilhada por índices de canto e preservação de UV/winding | Pendente |
| Paint 3D em n-gons côncavos | BROKEN | Projeção baricêntrica usava fan diferente do renderer | `face_hit_uv` usa `face_triangle_corners`; teste cobre o vazio de uma concavidade | Teste de domínio verde; visual pendente |
| Cor e conta-gotas Paint | BROKEN | UI Slint alterava `tools.paint_color`, enquanto textura usava `state.paint_color`; picker 3D lia a cor do primeiro vértice | Ambas as cores sincronizadas; picker 3D lê o pixel da textura composta | Testes Slint e domínio verdes; visual pendente |
| Seleção e hit UV | BROKEN | Shift desmarcava UV sem desmarcar a face 3D; clique no vazio deixava máscara residual; fan UV cobria concavidade | Estados 2D/3D sincronizados e hit UV por ear clipping | Testes Slint e domínio verdes; visual pendente |
| Fallback software | RUDIMENTARY | Shadings iguais; guias sem profundidade; grade incondicional | Textura/material/luz, opacidade, depth, hover e grade adaptativa | Pendente |
| X-Ray WGPU | PARTIALLY_COMPLIANT | Seleção conservava pipeline com profundidade | Pipelines próprios para seleção através | Pendente |
| Câmera e resize | BROKEN | Imagem atualizada sem reprojetar overlays | Atualização leve dos overlays e sincronização após primeiro layout | Pendente |
| Botões Duplicate/Delete | BROKEN | Sliders de brush sobrepostos | Sliders indevidos removidos | Pendente |
| Loop Cut Slide | BROKEN | Campo Slide chamava alteração de Cuts | Callback específico com validação numérica | Pendente |
| Clique com Move/Rotate/Scale | BROKEN | Pointer down iniciava operação imediatamente | Limiar de 4 px; clique continua seleção | Pendente |
| Hover e desempenho | PARTIALLY_COMPLIANT | Geometria reprocessada por mudança de hover/domínio | Buffers separados; contador de rebuild | Pendente |
| `.petunia` | PARTIALLY_COMPLIANT | ZIP/JSON já existe; contrato de manifest incompleto | Nenhuma migração inventada; compatibilidade preservada | Em análise |
| Connect | PARTIALLY_COMPLIANT | Apenas duas faces com mesma contagem | Algoritmo de cadeias abertas/anéis e contagens diferentes presente; preview de confirmação pendente | Pendente |
| Seleção múltipla/área | MISSING | Apenas um objeto ativo | Seleção contextual, Shift, retângulo, duplicação/exclusão em conjunto e contexto de Undo | Pendente |
| Transformação por ponteiro | BROKEN | Deslocamento e rotação dependiam de deltas aproximados | Interseção raio/plano/eixo, ângulo ao redor do pivô e precisão incremental | Pendente |
| Campo numérico | BROKEN | Clique iniciava operação; cancelamento não restaurava o rascunho | Draft, limiar de drag, confirmação/validação e cancelamento | Pendente |
| Histórico | BROKEN | Versão salva reutilizada após ramificar Undo | Identidades monotônicas e tamanho explícito no Undo/Redo | Pendente |
| Retomada dos checkpoints | MISSING | Objetos locais de 4364977/956eca0 ausentes no ambiente retomado | Código disponível preservado; diferenças registradas no plano sem alegação de recuperação integral | Aberto |
| Paridade global | PARTIALLY_COMPLIANT | Evidência insuficiente para declarar todas as features prontas | Plano de 30 itens com critérios de aceite | Aberto |
| Suíte Slint | BROKEN | 11 testes da sessão OpenCode falhavam por premissas antigas sobre domínio, alvos de aresta e sessão Cut | Provas atualizadas para seleção Object/Point, arestas projetadas e commit explícito | 169/169 verdes; interação manual pendente |

## Reconciliação das referências

O capítulo 05 descreve Textured/Silhouette e desaconselha Rendered; o anexo
posterior do usuário exige Wire/Solid/Material/Rendered reais. Nesta implementação
segue-se a instrução posterior do usuário, preservando a decisão explícita.
P3D-063 e capítulo 36 divergem na amplitude do workspace UV; essa divergência
permanece documentada e impede uma alegação automática de paridade global.
O pedido atual de avançar UV orienta a implementação, mas a decisão formal do
adendo de 16/09 sobre a pill UV ainda precisa ser reconciliada na fonte canônica.
