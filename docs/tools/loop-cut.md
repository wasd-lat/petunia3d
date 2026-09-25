# Ferramenta: Corte em Anel (Loop Cut)

O **Loop Cut** analisa as conexões de quadriláteros na malha e insere uma linha contínua de arestas transversais.

- **Atalho de Ativação**: `Ctrl+R`
- **Disponível em**: componentes da malha (`Face` / `Edge` / `Point`); detalhe técnico: `EditMode::Edit`.

## Como Usar
1. Pressione `Ctrl+R` ou selecione **Loop Cut** na barra de ferramentas;
2. Aproxime o cursor de uma aresta: uma linha-guia amarela indicará a prévia do corte perpendicular no anel de quadriláteros;
3. Use a **roda do mouse** (scroll) para ajustar interativamente a contagem de cortes (mínimo 1, ou mínimo 2 se o modo balanceado estiver ativo);
4. Clique com o botão esquerdo (`LMB`): o corte é gerado e entra no modo de deslizamento (*slide*);
5. Deslize o mouse para posicionar o anel exatamente onde deseja;
6. Clique novamente com `LMB` para fixar, ou pressione `Escape` para centralizar perfeitamente no meio da face.

## Modo Dual Balanced (Cortes Duplamente Balanceados)
O ToolCard do Loop Cut na viewport oferece a opção **Dual Balanced**:
- Posiciona pares de cortes simetricamente distribuídos em relação ao ponto médio (0.5) do anel;
- O deslizamento (*slide*) afasta ou aproxima os cortes espelhadamente em direção às bordas ou ao centro;
- Força automaticamente o mínimo de 2 cortes ao ser ativado, mantendo a simetria perfeita para modelagem de chanfros de apoio (support loops) e subdivisão controlada.
