# Catálogo de Ferramentas

O Petunia3D possui uma suíte concisa e poderosa de 19 ferramentas dedicadas para criação e manipulação geométrica.

---

## 🧭 Ferramentas de Seleção e Transformação

| Ferramenta | Atalho | Modos Suportados | Descrição |
| :--- | :--- | :--- | :--- |
| **[Seleção (Select)](./select)** | `W` / `B` | Objeto & Edição | Seleção por clique, múltipla com `Shift` e seleção por caixa (`Box Select`). |
| **[Mover (Move)](./move)** | `G` | Objeto & Edição | Translação livre ou travada em eixos `X`, `Y`, `Z` ou planos `Shift+X/Y/Z`. |
| **[Rotacionar (Rotate)](./rotate)** | `R` | Objeto & Edição | Rotação angular em torno da normal da vista ou eixos coordenados. |
| **[Escalar (Scale)](./scale)** | `S` | Objeto & Edição | Escala uniforme ou anisotrópica ao longo de eixos e planos. |
| **[Gizmo Combinado (Transform)](./transform)** | `T` | Objeto & Edição | Gizmo integrado com alças de translação, anéis de rotação e cubos de escala. |

---

## 🛠️ Ferramentas de Modelagem de Malha (componentes: Face / Edge / Point)

| Ferramenta | Atalho | Descrição |
| :--- | :--- | :--- |
| **[Extrusão (Extrude)](./extrude)** | `E` | Projeta faces ou arestas ao longo de normais ou eixos travados. |
| **[Inserção (Inset)](./inset)** | `I` | Cria anéis de faces recuadas interiormente. |
| **[Round Edge (Bevel)](./bevel)** | `Ctrl+B` | Arredonda quinas e arestas com facetas limpas (1 segmento no Core V1). |
| **[Corte em Anel (Loop Cut)](./loop-cut)** | `Ctrl+R` | Insere loops de arestas contínuos com deslizamento interativo. |
| **[Faca (Knife)](./knife)** | `K` | Desenha cortes livres conectando vértices e arestas. |
| **[Push / Pull](./push-pull)** | `Shift+P` | Empurra ou puxa faces preservando a orientação original. |
| **[Fatiamento (Slice)](./slice)** | `Shift+K` | Corta a malha inteira através de um plano infinito desenhado. |
| **[Subdivisão (Subdivide)](./subdivide)** | `Ctrl+D` | Subdivide faces selecionadas em subquads uniformes. |
| **[Desenhar Perfil (Draw Profile)](./draw-profile)** | `Shift+D` | Desenha contornos 2D para geração de formas tridimensionais. |

---

## 📏 Medição, Anotação e Auxiliares 3D

| Ferramenta | Atalho | Descrição |
| :--- | :--- | :--- |
| **[3D Cursor](./cursor3d)** | `Shift+RMB` | Posiciona a âncora 3D para criação de primitivas e pivôs. |
| **[Régua 3D (Measure)](./measure)** | `M` | Medição de distâncias euclidianas e deltas cartesianos com snap a vértices. |
| **[Anotação 3D (Annotate)](./annotate)** *(Pós-V1)* | `—` | Rascunhos grease-pencil sobre a geometria (planejada para a Era 1 pós-GA). |
| **[Adicionar Primitiva (Primitive)](./primitive)** | `Shift+A` | Instancia cubos, esferas, cilindros e planos no 3D Cursor. |
| **[Imagens de Referência](./reference-image)** | `Shift+I` | Carrega imagens de referência ortogonais no viewport. |
