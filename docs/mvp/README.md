# Petunia3D — Contratos de Implementação do MVP

Esta pasta contém as **diretivas de implementação vigentes** para levar o Petunia3D
ao estado de MVP realmente utilizável e, depois, a Release Candidate.

## Autoridade

1. `docs/bible/` continua sendo o caderno canônico de produto, arquitetura e
   vocabulário. Em conflito com esta pasta, o caderno prevalece.
2. Dentro do escopo de implementação de UI/UX/interação/ferramentas/comportamento
   do MVP, os documentos desta pasta **prevalecem sobre** a UI atualmente
   implementada, código legado, componentes antigos, mockups antigos e
   comportamentos improvisados.
3. Quando uma restrição estrutural comprovada do core impedir tecnicamente um
   comportamento exigido aqui, é obrigatório: não inventar comportamento
   alternativo em silêncio, documentar a limitação, implementar a melhor
   integração possível, marcar como `PARTIAL` e informar arquivo/função/limitação
   no relatório final.

## Documentos

| Arquivo | Conteúdo |
|---|---|
| `00-master-implementation-contract.md` | Contrato mestre de UI/UX/interação/ferramentas/comportamento/funcionalidade do MVP. |
| `01-paint-uv-workspace-redesign.md` | Especificação de redesenho dos workspaces PAINT e UV. Substitui a implementação atual desses dois workspaces. |
| `02-final-gauntlet.md` | Adendo obrigatório de auditoria, teste, inspeção, crítica e regressão. Revoga, nesta rodada, a restrição anterior de não executar testes. |
| `03-interaction-manifest.md` | Inventário obrigatório de elementos interativos e regras de cobertura. |
| `04-issue-ledger.md` | Ledger de issues do Gauntlet. |
| `05-scorecard.md` | Scorecard final por área. |

## Regra central

**ZERO FAKE UI.** Um controle não está implementado porque aparece. Uma
ferramenta só é `COMPLETE` quando o fluxo relevante alcança o estado real do
documento:

```
UI
→ Command
→ Tool Controller
→ Application/Domain
→ Document Mutation
→ Undo Transaction
→ Renderer Update
→ UI State Update
→ Save
→ Reload
```

É proibido considerar `COMPLETE` algo contendo `TODO`, `FIXME`, `todo!()`,
`unimplemented!()`, panic como placeholder, mock, stub, noop, `println` como
implementação, callback vazio, estatística falsa, preview falso ou resultado
hardcoded.

## Rotina do Gauntlet

```
DISCOVER → AUDIT → IMPLEMENT → BUILD → STATIC ANALYSIS → UNIT TEST
→ PROPERTY/EDGE TEST → INTEGRATION TEST → UI INTERACTION TEST
→ MANUAL USABILITY PASS → VISUAL REVIEW → ACCESSIBILITY REVIEW
→ PERFORMANCE REVIEW → SECURITY REVIEW → ARCHITECTURE REVIEW
→ REGRESSION → SCORE → FIX
```

Repetir até que todas as categorias obrigatórias atinjam `10/10`. Não existe
número fixo de loops. Cada `10/10` exige evidência, nunca impressão. Não é
permitido declarar `10/10` com qualquer teste falhando, ferramenta `MISSING`,
item `PARTIAL`, elemento interativo `NOT TESTED`, defeito conhecido no escopo,
perda de dados em Save/Load ou placeholder funcional exposto.
