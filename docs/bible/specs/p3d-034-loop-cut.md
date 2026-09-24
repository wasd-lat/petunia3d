# P3D-034 — Loop Cut

<aside>
🧩

Estado: **parcialmente implementado; UX por hover adicionada em 2026-09-23** · Prioridade: P1.

</aside>

## Objetivo

Inserir edge loops em topologia compatível sem prometer resultado onde não há caminho de loop válido.

## Auditoria

Validar detecção do loop, preview, posição/slide se existir, seleção resultante e undo.

## Regras

Em topologia incompatível, ação fica disabled ou retorna erro/hint claro; não “chuta” um corte.

## Interação da UI Slint

Escolher Loop Cut arma a ferramenta. Passar o ponteiro sobre uma aresta de um
quad ring elegível desenha uma prévia sem mutar a malha. A roda ajusta `Cuts`
(1–32) sem acionar zoom. Clique inicia a sessão transacional para slide; Enter
confirma, Esc cancela. Rings não suportados não produzem preview e informam a
restrição. O preview nativo ainda precisa de QA visual e comparação de
oclusão/topologias complexas.

## Dependências

P3D-018, P3D-041, P3D-123.

## Testes / DoD

Quads regulares, boundaries, poles/ngons, cancel e regressão.
