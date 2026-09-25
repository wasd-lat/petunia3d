# Ferramenta: Desenhar Perfil (Draw Profile)

A ferramenta **Draw Profile** permite desenhar contornos e silhuetas vetoriais bidimensionais sobre planos de referência ou vistas ortográficas.

- **Atalho de Ativação**: `Shift+D`
- **Disponível em**: componentes da malha (detalhe técnico: `EditMode::Edit`) e vistas ortográficas (`Numpad 1, 3, 7`).

## Aplicações
- Desenhar silhuetas a partir de imagens de referência ("blueprints");
- Geração de peças de revolução (taças, garrafas, colunas) ou extrusão direta de volume.

## Recursos Avançados de Curvas e Paredes

### Sistema Bézier Completo
- **Nós e Alças (Handles)**: O perfil suporta nós poligonais e nós com curvas Bézier cúbicas contínuas (alças de controle tangentes G1/C1).
- **Suavizar Curvas / Cantos Retos**: A ToolCard permite alternar instantaneamente entre nós retos e nós suavizados com alças calculadas automaticamente com base na curvatura dos vértices adjacentes.
- **Tesselação Adaptativa De Casteljau**: As curvas são amostradas dinamicamente com base no parâmetro de suavização, garantindo contornos suaves sem excesso desnecessário de polígonos.

### Hollow Profile (Espessura de Parede)
- **Perfis Ocos Paramétricos**: Através do campo **Espessura de Parede (Wall Thickness)**, qualquer perfil fechado gera automaticamente um contorno duplo equidistante voltado para o interior com miter clamping de cantos.
- **Visualização em Tempo Real**: O viewport exibe o laço externo e o contorno interno em tempo real antes da confirmação.
- **Extrusão e Revolve Ocos**: Ao acionar `Extrude` ou `Revolve`, a geometria resultante é gerada com espessura de casca sólida sem necessidade de modificadores posteriores.

### Varredura 3D (Sweep) com Rotation Minimizing Frames (RMF)
- **Varredura ao Longo de Curvas Guias**: Permite varrer qualquer perfil 2D (aberto ou fechado, sólido ou oco com espessura de parede) ao longo de uma espinha 3D (*spine*).
- **Transporte Paralelo e Eliminação de Gimbal Lock**: Utiliza a técnica de *Double Reflection RMF* (Wang et al., 2008), eliminando torções anômalas (*flipping*) e garantindo continuidade C2 ao longo de todo o caminho 3D.
- **Compensação de Torção em Laços Fechados**: Distribui uniformemente qualquer rotação residual ao longo do comprimento quando o caminho guia é fechado.
- **Mitering e Alinhamento Bissetor em Cantos Vivos**: Alinha automaticamente as seções transversais ao plano bissetor em quinas pronunciadas com limitação de extensão (*miter limit clamping*), evitando pontas infinitas e deformações.
- **Seleção Flexível de Caminho**: O Sweep utiliza automaticamente cadeias de arestas ou vértices selecionados na malha ativa como curva guia, ou uma curva 3D suave padrão caso não haja seleção.
- **Tampas Trianguladas (End Caps)**: Gera tampas planas trianguladas no início e fim de varreduras em caminhos abertos.

