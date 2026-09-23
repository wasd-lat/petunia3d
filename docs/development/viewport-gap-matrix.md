# Implementação versus especificação — viewport

Base: `f94b888`. Atualização: 22/09/2026. Fontes: Livro Vivo local,
`Markdown(2).md colado` e três capturas anexadas pelo usuário.

A instrução recente do usuário prevalece sobre a ordem dos gates: implementar
primeiro, testar ao terminar. Estados abaixo descrevem inspeção e mudanças de
código, sem alegação de validação executável.

| Fluxo | Base | Causa observada | Implementação neste ramo | Aceite |
|---|---|---|---|---|
| Contorno, marcadores e gizmos | BROKEN | Slint Path usa ajuste ao bounding box em coordenadas já projetadas | `ImageFit.preserve`; tripé em coordenadas locais e clipping | Pendente |
| Troca de seleção | BROKEN | Frame não redesenhado e flags de domínio residual | Conversão de seleção, limpeza de hover e redraw no callback | Pendente |
| Picking | PARTIALLY_COMPLIANT | Bounding sphere e tolerância em mundo | Raycast de faces; alvos em pixels; hover/clique usam mesmo picker | Pendente |
| Oclusão | PARTIALLY_COMPLIANT | Raio do cursor usado para candidato deslocado | Raio pelo candidato em perspectiva e ortográfica | Pendente |
| Polígonos côncavos | BROKEN | Fan cobre regiões exteriores do polígono | Triangulação compartilhada por índices de canto e preservação de UV/winding | Pendente |
| Fallback software | RUDIMENTARY | Shadings iguais; guias sem profundidade; grade incondicional | Textura/material/luz, opacidade, depth, hover e grade adaptativa | Pendente |
| X-Ray WGPU | PARTIALLY_COMPLIANT | Seleção conservava pipeline com profundidade | Pipelines próprios para seleção através | Pendente |
| Câmera e resize | BROKEN | Imagem atualizada sem reprojetar overlays | Atualização leve dos overlays e sincronização após primeiro layout | Pendente |
| Botões Duplicate/Delete | BROKEN | Sliders de brush sobrepostos | Sliders indevidos removidos | Pendente |
| Loop Cut Slide | BROKEN | Campo Slide chamava alteração de Cuts | Callback específico com validação numérica | Pendente |
| Clique com Move/Rotate/Scale | BROKEN | Pointer down iniciava operação imediatamente | Limiar de 4 px; clique continua seleção | Pendente |
| Hover e desempenho | PARTIALLY_COMPLIANT | Geometria reprocessada por mudança de hover/domínio | Buffers separados; contador de rebuild | Pendente |
| `.petunia` | PARTIALLY_COMPLIANT | ZIP/JSON já existe; contrato de manifest incompleto | Nenhuma migração inventada; compatibilidade preservada | Em análise |
| Connect | PARTIALLY_COMPLIANT | Apenas duas faces com mesma contagem | Loops abertos e contagens diferentes pendentes | Em implementação |
| Paridade global | PARTIALLY_COMPLIANT | Evidência insuficiente para declarar todas as features prontas | Plano de 30 itens com critérios de aceite | Aberto |

## Reconciliação das referências

O capítulo 05 descreve Textured/Silhouette e desaconselha Rendered; o anexo
posterior do usuário exige Wire/Solid/Material/Rendered reais. Nesta implementação
segue-se a instrução posterior do usuário, preservando a decisão explícita.
P3D-063 e capítulo 36 divergem na amplitude do workspace UV; essa divergência
permanece documentada e impede uma alegação automática de paridade global.
