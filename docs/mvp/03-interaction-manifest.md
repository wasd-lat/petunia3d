# Interaction Manifest — Petunia3D MVP

> Inventário obrigatório de **todo** elemento interativo visível no MVP.
> Nenhum elemento pode permanecer `UNTRACKED`, `UNKNOWN`, `NOT TESTED` ou
> `ASSUMED WORKING`.

## Nomenclatura

Identificadores estáveis no formato `AREA.GRUPO.ELEMENTO`, por exemplo:

```
APP.FILE.NEW
APP.FILE.OPEN
APP.FILE.SAVE
APP.WORKSPACE.MODEL
APP.WORKSPACE.PAINT
APP.WORKSPACE.UV
APP.COMMAND.UNDO
APP.COMMAND.REDO
APP.COMMAND.SEARCH
APP.COMMAND.SETTINGS
MODEL.TOOL.SELECT
MODEL.TOOL.MOVE
MODEL.TOOL.EXTRUDE
MODEL.OUTLINER.VISIBILITY
MODEL.OUTLINER.LOCK
MODEL.SPLITTER.OUTLINER
MODEL.SPLITTER.INSPECTOR
PAINT.TOOL.BRUSH
PAINT.LAYER.OPACITY
PAINT.CANVAS.ZOOM
UV.TOOL.UNWRAP
UV.ISLAND.MOVE
UV.SPLITTER.CENTER
DIALOG.UNSAVED.SAVE
PREFERENCES.THEME.SELECT
```

## Campos obrigatórios por item

| Campo | Descrição |
|---|---|
| `ID` | Identificador estável. |
| `Workspace` | `GLOBAL`, `MODEL`, `PAINT`, `UV`. |
| `Location` | Região/arquivo onde vive. |
| `Element Type` | Button, IconButton, MenuItem, Field, Toggle, Splitter, … |
| `Label Token` | Token de texto público. |
| `Tooltip Token` | Token do tooltip. |
| `Icon ID` | Semantic icon id. |
| `Command ID` | CommandId canônico. |
| `Shortcut` | Atalho resolvido pelo keymap. |
| `Enabled Condition` | Condição de `can_execute`. |
| `Disabled Reason` | Motivo apresentável. |
| `Input` | Mouse, teclado, drag, wheel. |
| `Expected Behavior` | Comportamento observável. |
| `State Mutation` | O que muda no documento. |
| `Visual Feedback` | Estados visuais. |
| `Undo Requirement` | Agrupa em uma operação? |
| `Persistence Requirement` | Sobrevive a Save/Load? |
| `Accessibility Label` | Nome acessível. |
| `Automated Test` | Teste que cobre. |
| `Manual Test` | Roteiro manual. |
| `Status` | `COMPLETE` / `PARTIAL` / `STUB` / `BROKEN` / `MISSING`. |
| `Evidence` | Arquivo:linha, teste, screenshot. |

## Regra de cobertura

100 % dos elementos interativos visíveis no MVP precisam existir neste
manifesto. O manifesto é preenchido durante a FASE 1 (auditoria) e atualizado a
cada loop do Gauntlet. O relatório final inclui os totais de elementos
descobertos, implementados, testados automaticamente, testados manualmente,
não verificados, quebrados, stub e mortos.
