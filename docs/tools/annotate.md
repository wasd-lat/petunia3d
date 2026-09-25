# Ferramenta: Anotação 3D (Annotate)

> **Status de Versão:** Planejada para pós-V1 (Era 1.x / Roadmap). No frontend Slint de produção da V1, o atalho `D` é dedicado a funções de navegação e manipulação de pivô, e a estrutura de dados de anotações livres em espaço 3D permanece preservada no Core/Project (`ProjectDocument.annotations`) para a expansão pós-GA.

A ferramenta de **Anotação 3D** funciona como um lápis de rascunho (*grease-pencil*) tridimensional integrado, permitindo desenhar anotações e guias visuais diretamente no espaço da cena.

- **Atalho de Ativação**: `—` (planejado para a Era 1.x).
- **Desenho**: Segure `D` + arraste com `LMB` (ou selecione a ferramenta e desenhe diretamente com `LMB`).

## Recursos Avançados
- **Coleção Exclusiva no Outliner**: Todas as anotações são obrigatoriamente salvas na coleção `📝 Anotações` no topo do Outliner (com ícone ciano `#00d2d3`), com confinamento estrito impedindo que se misturem com malhas poligonais.
- **Subgrupos Internos**: Suporte a criar subgrupos (`📁 Subgrupo`) para organizar diferentes etapas de anotação (ex: "Estrutura", "Correções", "Ajuste de Proporção").
- **Inspetor de Propriedades**: Selecionar uma anotação exibe no painel lateral direito seus controles de cor RGBA, espessura do traço e seção completa de Transformação (Posição X/Y/Z, Rotação em graus e Escala).
- **Manipulação por Gizmo 3D**: Cada traço possui centro geométrico e pode ser movido, rotacionado e escalado com gizmos 3D interativos no viewport.
- **Desfazer / Refazer Total (`Ctrl+Z` / `Ctrl+Shift+Z`)**: Checkpoints automáticos a cada traço finalizado.
