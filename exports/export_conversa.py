#!/usr/bin/env python3
import json
import os
import re
import shutil
from datetime import datetime

src_dir = '/home/raillen/.gemini/antigravity-ide/brain/3e057553-4ee5-4f44-a011-a9c1aaa46f90'
out_dir = '/home/raillen/Documentos/petunia3d/exports/conversa-antigravity-ide'
os.makedirs(os.path.join(out_dir, 'imagens'), exist_ok=True)
os.makedirs(os.path.join(out_dir, 'artefatos'), exist_ok=True)

# 1. Copiar artefatos
for f in ['deep_ui_analysis.md', 'petunia_ui_modernization_plan.md', 'shell_modernization_plan.md']:
    p = os.path.join(src_dir, f)
    if os.path.exists(p):
        shutil.copy2(p, os.path.join(out_dir, 'artefatos', f))

# 2. Copiar imagens
img_src_dir = os.path.join(src_dir, '.user_uploaded')
copied_images = []
if os.path.exists(img_src_dir):
    for img in sorted(os.listdir(img_src_dir)):
        if img.endswith('.png') or img.endswith('.jpg'):
            shutil.copy2(os.path.join(img_src_dir, img), os.path.join(out_dir, 'imagens', img))
            copied_images.append(img)

# 3. Ler transcript
transcript_path = os.path.join(src_dir, '.system_generated', 'logs', 'transcript_full.jsonl')
with open(transcript_path, 'r', encoding='utf-8') as f:
    steps = [json.loads(line) for line in f]

image_mapping = {
    0: ['media_1790280453835.png', 'media_1790280516812.png'],
    7: ['media_1790280711634.png'],
    53: ['media_1790281381447.png'],
    477: ['media_1790287693208.png']
}

image_captions = {
    'media_1790280453835.png': 'Interface A: CAD moderno (estilo Plasticity) com robô esférico',
    'media_1790280516812.png': 'Interface B: Cinema 4D clássico com cubo azul e painéis densos',
    'media_1790280711634.png': 'Captura do Petunia3D com painel direito (Inspector) e viewport',
    'media_1790281381447.png': 'Captura da barra de ferramentas e viewport com viewport_floating_bar',
    'media_1790287693208.png': 'Captura do painel flutuante de propriedades de ferramentas quebrado'
}

# 4. Arquivos modificados
modified_files = set()
for s in steps:
    for tc in s.get('tool_calls', []):
        fn = tc.get('name')
        args = tc.get('args', {})
        if fn in ['replace_file_content', 'write_to_file']:
            target = args.get('TargetFile') or args.get('target_file') or args.get('path') or ''
            if target and 'petunia3d' in target and not 'exports' in target:
                rel = target.split('petunia3d/')[-1]
                modified_files.add(rel)

lines = []
lines.append('# Exportação Completa da Conversa — Antigravity IDE')
lines.append('')
lines.append('> **Projeto:** Petunia3D (`wasd-lat/petunia3d`)  ')
lines.append('> **Sessão ID:** `3e057553-4ee5-4f44-a011-a9c1aaa46f90`  ')
lines.append('> **Data de Início:** 2026-09-24 17:08:45 (-03:00) / 20:08:45 UTC  ')
lines.append('> **Data de Término:** 2026-09-24 19:30:41 (-03:00) / 22:30:41 UTC  ')
lines.append('> **Total de Passos Registrados:** 642 passos  ')
lines.append('> **Ambiente de Origem:** Google Antigravity IDE (`~/.gemini/antigravity-ide/`)  ')
lines.append('')
lines.append('---')
lines.append('')
lines.append('## 📑 Sumário')
lines.append('')
lines.append('1. [Resumo Executivo da Sessão](#1-resumo-executivo-da-sessão)')
lines.append('2. [Artefatos Técnicos Produzidos](#2-artefatos-técnicos-produzidos)')
lines.append('3. [Imagens e Capturas Compartilhadas](#3-imagens-e-capturas-compartilhadas)')
lines.append('4. [Arquivos Modificados no Repositório](#4-arquivos-modificados-no-repositório)')
lines.append('5. [Transcrição Cronológica Completa da Conversa](#5-transcrição-cronológica-completa-da-conversa)')
lines.append('')
lines.append('---')
lines.append('')
lines.append('## 1. Resumo Executivo da Sessão')
lines.append('')
lines.append('Esta sessão de desenvolvimento no Antigravity IDE foi dedicada à modernização visual e arquitetural da interface de produção do **Petunia3D** em **Slint** (`crates/ui-slint/`).')
lines.append('')
lines.append('Os principais marcos foram:')
lines.append('- **Análise Comparativa Profunda:** O usuário enviou duas interfaces de referência (uma moderna no estilo Plasticity com modelagem direta e outra densa no estilo Cinema 4D). O assistente realizou uma decomposição detalhada de ergonomia, hierarquia visual e densidade de informação, gerando o artefato `deep_ui_analysis.md`.')
lines.append('- **Estratégia de UI para Petunia3D:** Definição de 4 eixos de evolução: Viewport-first imersivo, desacoplamento e independência dos módulos do painel direito (Inspector), barra flutuante de ferramentas e simplificação do shelf inferior (`petunia_ui_modernization_plan.md`).')
lines.append('- **Planejamento Detalhado por Áreas (Shell):** Elaboração e aprovação de plano cobrindo Header, Barra de Ferramentas esquerda, Barra flutuante de viewport, Shelf de rodapé, Status Bar e Painel flutuante de opções de ferramenta (`shell_modernization_plan.md`).')
lines.append('- **Implementação em Slint:** Ajustes no arquivo declarativo `crates/ui-slint/ui/app.slint` e bridges Rust correspondentes.')
lines.append('- **Correção Crítica do Painel de Ferramentas:** O usuário enviou uma captura mostrando que a janela flutuante de opções das ferramentas estava quebrada e com layout inconsistente. O assistente corrigiu a estrutura de layout Slint, espaçamentos, títulos e campos de entrada.')
lines.append('- **Validação e Gates de Qualidade:** Todos os testes unitários (`cargo test -p petunia_ui_slint --lib`), testes de integração e verificações rigorosas do `cargo clippy --all-targets -- -D warnings` foram aprovados.')
lines.append('')
lines.append('---')
lines.append('')
lines.append('## 2. Artefatos Técnicos Produzidos')
lines.append('')
lines.append('Os seguintes artefatos foram gerados durante a sessão e estão preservados na pasta `artefatos/`:')
lines.append('')
lines.append('- 📄 **[deep_ui_analysis.md](./artefatos/deep_ui_analysis.md)**: Análise comparativa aprofundada de design e ergonomia.')
lines.append('- 📄 **[petunia_ui_modernization_plan.md](./artefatos/petunia_ui_modernization_plan.md)**: Plano de modernização da UI em Slint (4 eixos fundamentais).')
lines.append('- 📄 **[shell_modernization_plan.md](./artefatos/shell_modernization_plan.md)**: Plano arquitetural e de layout das 6 zonas da interface Petunia3D.')
lines.append('')
lines.append('---')
lines.append('')
lines.append('## 3. Imagens e Capturas Compartilhadas')
lines.append('')
for img_name, cap in image_captions.items():
    lines.append(f'- **{cap}**  ')
    lines.append(f'  ![{cap}](./imagens/{img_name})')
lines.append('')
lines.append('---')
lines.append('')
lines.append('## 4. Arquivos Modificados no Repositório')
lines.append('')
for mf in sorted(modified_files):
    lines.append(f'- `{mf}`')
lines.append('')
lines.append('---')
lines.append('')
lines.append('## 5. Transcrição Cronológica Completa da Conversa')
lines.append('')

for idx, s in enumerate(steps):
    stype = s.get('type')
    source = s.get('source')
    content = s.get('content', '')
    created_at = s.get('created_at', '')

    if stype == 'USER_INPUT' or source == 'USER_EXPLICIT':
        clean_msg = re.sub(r'<ADDITIONAL_METADATA>.*?</ADDITIONAL_METADATA>', '', content, flags=re.DOTALL)
        clean_msg = re.sub(r'<USER_SETTINGS_CHANGE>.*?</USER_SETTINGS_CHANGE>', '', clean_msg, flags=re.DOTALL)
        clean_msg = re.sub(r'</?USER_REQUEST>', '', clean_msg).strip()

        lines.append(f'### 👤 Usuário — Passo {idx} ({created_at})')
        lines.append('')
        if clean_msg:
            lines.append(clean_msg)
            lines.append('')

        if idx in image_mapping:
            for img in image_mapping[idx]:
                cap = image_captions.get(img, img)
                lines.append(f'![{cap}](./imagens/{img})')
                lines.append(f'*Figura: {cap}*')
                lines.append('')

        lines.append('---')
        lines.append('')

    elif stype == 'PLANNER_RESPONSE':
        tool_calls = s.get('tool_calls', [])
        if content and content.strip():
            lines.append(f'### 🤖 Assistente — Passo {idx} ({created_at})')
            lines.append('')
            lines.append(content.strip())
            lines.append('')
            lines.append('---')
            lines.append('')
        elif tool_calls:
            actions = []
            for tc in tool_calls:
                name = tc.get('name')
                args = tc.get('args', {})
                if name == 'write_to_file':
                    target = args.get('TargetFile', '')
                    actions.append(f'Criou artefato/arquivo: `{os.path.basename(target)}`')
                elif name == 'replace_file_content':
                    target = args.get('TargetFile', '')
                    actions.append(f'Editou arquivo: `{os.path.basename(target)}` ({args.get("Description", "")})')
                elif name == 'run_command':
                    cmd = args.get('CommandLine', '')
                    if any(k in cmd for k in ['cargo test', 'cargo clippy', 'cargo check', 'git ']):
                        actions.append(f'Executou comando: `{cmd}`')
            if actions:
                lines.append(f'> ⚙️ *Ações no passo {idx}:*')
                for act in actions:
                    lines.append(f'> - {act}')
                lines.append('')

md_path = os.path.join(out_dir, 'conversa_completa.md')
with open(md_path, 'w', encoding='utf-8') as f:
    f.write('\n'.join(lines))

print(f'Sucesso! Exportado para {md_path} ({len(lines)} linhas, {os.path.getsize(md_path)} bytes)')
