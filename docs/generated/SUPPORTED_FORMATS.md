---
title: Formatos 3D Suportados & Capacidades
description: Matriz de capacidades do pipeline de entrega e importadores/exportadores (P3D-119)
---

<!--
  ARQUIVO GERADO AUTOMATICAMENTE — NÃO EDITE MANUALMENTE!
  Gerado deterministicamente por `cargo xtask docs` (P3D-119).
  Para atualizar execute: cargo run -p xtask -- docs
-->

# Matriz de Formatos 3D Suportados (`DeliveryPipeline`)

> **Single Source of Truth (P3D-068 a P3D-072, P3D-124, P3D-119)**
> O Petunia3D adota um pipeline modular desacoplado da interface gráfica para leitura e emissão de modelos 3D.

## Matriz de Capacidades por Formato

| Formato | Extensão | Importação | Exportação | Materiais PBR | Texturas | Cores p/ Vértice | Múltiplas Malhas | Formato Binário |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **Wavefront OBJ** | `.obj` | ✅ Sim | ✅ Sim | — | — | — | — | — |
| **glTF 2.0 (JSON)** | `.gltf` | ✅ Sim | ❌ Não | ✅ | ✅ | ✅ | ✅ | — |
| **glTF 2.0 Binary (GLB)** | `.glb` | ✅ Sim | ✅ Sim | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Petunia Package (.pkg)** | `.pkg` | ✅ Sim | ✅ Sim | ✅ | ✅ | ✅ | ✅ | ✅ |


## Opções do Pipeline

- **Triangulação (`triangulate`)**: Converte faces poligonais em triângulos no momento da emissão.
- **Exportação de Materiais (`export_materials`)**: Emite propriedades PBR (Base Color, Roughness, Metallic, Emission).
- **Fator de Escala (`scale`)**: Transforma dimensões geométricas uniformemente durante importação ou exportação.
- **Tolerância a Falhas em Lote (`batch_export`)**: Falhas parciais em assets individuais não interrompem os demais.

