//! Testes de fitness arquitetural (Gauntlet G0 e G1).
//! Garante que o núcleo da aplicação (core, config, mesh, project, commands, render)
//! permaneça 100% puro e desacoplado de dependências de apresentação (egui).

use std::fs;
use std::path::Path;

fn root_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn petunia_core_must_not_depend_on_egui() {
    let manifest_path = root_dir().join("crates/core/Cargo.toml");
    let content = fs::read_to_string(&manifest_path).expect("crates/core/Cargo.toml");
    assert!(
        !content.contains("egui ="),
        "VIOLAÇÃO ARQUITETURAL: crates/core/Cargo.toml não pode depender de egui!"
    );
}

#[test]
fn petunia_config_must_not_depend_on_egui() {
    let manifest_path = root_dir().join("crates/config/Cargo.toml");
    let content = fs::read_to_string(&manifest_path).expect("crates/config/Cargo.toml");
    assert!(
        !content.contains("egui ="),
        "VIOLAÇÃO ARQUITETURAL: crates/config/Cargo.toml não pode depender de egui!"
    );
}

#[test]
fn petunia_mesh_must_be_pure() {
    let manifest_path = root_dir().join("crates/mesh/Cargo.toml");
    let content = fs::read_to_string(&manifest_path).expect("crates/mesh/Cargo.toml");
    assert!(
        !content.contains("egui"),
        "VIOLAÇÃO ARQUITETURAL: crates/mesh/Cargo.toml não pode depender de egui!"
    );
    assert!(
        !content.contains("petunia_core"),
        "VIOLAÇÃO ARQUITETURAL: crates/mesh/Cargo.toml não pode depender de petunia_core!"
    );
    assert!(
        !content.contains("petunia_ui"),
        "VIOLAÇÃO ARQUITETURAL: crates/mesh/Cargo.toml não pode depender de petunia_ui!"
    );
}

#[test]
fn petunia_project_must_not_depend_on_ui() {
    let manifest_path = root_dir().join("crates/project/Cargo.toml");
    let content = fs::read_to_string(&manifest_path).expect("crates/project/Cargo.toml");
    assert!(
        !content.contains("egui"),
        "VIOLAÇÃO ARQUITETURAL: crates/project/Cargo.toml não pode depender de egui!"
    );
    assert!(
        !content.contains("petunia_ui"),
        "VIOLAÇÃO ARQUITETURAL: crates/project/Cargo.toml não pode depender de petunia_ui!"
    );
}

#[test]
fn petunia_commands_must_not_depend_on_ui() {
    let manifest_path = root_dir().join("crates/commands/Cargo.toml");
    let content = fs::read_to_string(&manifest_path).expect("crates/commands/Cargo.toml");
    assert!(
        !content.contains("egui"),
        "VIOLAÇÃO ARQUITETURAL: crates/commands/Cargo.toml não pode depender de egui!"
    );
    assert!(
        !content.contains("petunia_ui"),
        "VIOLAÇÃO ARQUITETURAL: crates/commands/Cargo.toml não pode depender de petunia_ui!"
    );
}

#[test]
fn petunia_render_wgpu_must_not_depend_on_egui() {
    let manifest_path = root_dir().join("crates/render-wgpu/Cargo.toml");
    let content = fs::read_to_string(&manifest_path).expect("crates/render-wgpu/Cargo.toml");
    assert!(
        !content.contains("egui"),
        "VIOLAÇÃO ARQUITETURAL: crates/render-wgpu/Cargo.toml não pode depender de egui!"
    );
    assert!(
        !content.contains("petunia_ui"),
        "VIOLAÇÃO ARQUITETURAL: crates/render-wgpu/Cargo.toml não pode depender de petunia_ui!"
    );
}

#[test]
fn petunia_module_paint_must_not_depend_on_rfd() {
    let manifest_path = root_dir().join("crates/module-paint/Cargo.toml");
    let content = fs::read_to_string(&manifest_path).expect("crates/module-paint/Cargo.toml");
    assert!(
        !content.contains("rfd ="),
        "VIOLAÇÃO ARQUITETURAL: crates/module-paint/Cargo.toml não pode depender de rfd!"
    );
}

#[test]
fn file_dialogs_isolated_strictly_to_file_dialog_service() {
    let crates_dir = root_dir().join("crates");
    let mut rs_files = Vec::new();

    fn scan_dir(dir: &Path, acc: &mut Vec<std::path::PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    scan_dir(&path, acc);
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    acc.push(path);
                }
            }
        }
    }

    scan_dir(&crates_dir, &mut rs_files);

    for file_path in rs_files {
        let rel_path = file_path
            .strip_prefix(root_dir())
            .unwrap_or(&file_path)
            .to_string_lossy();

        if rel_path == "crates/ui/src/file_dialog_service.rs"
            || rel_path.starts_with("crates/xtask")
        {
            continue;
        }

        let content = fs::read_to_string(&file_path).expect("ler arquivo fonte");
        assert!(
            !content.contains("rfd::FileDialog"),
            "VIOLAÇÃO ARQUITETURAL (G4): {rel_path} instancia rfd::FileDialog fora de file_dialog_service.rs!"
        );
        assert!(
            !content.contains("egui_file_dialog::FileDialog"),
            "VIOLAÇÃO ARQUITETURAL (G4): {rel_path} instancia egui_file_dialog fora de file_dialog_service.rs!"
        );
    }
}

#[test]
fn tool_and_pointer_sessions_isolated_from_egui_memory() {
    let crates_dir = root_dir().join("crates");
    let mut rs_files = Vec::new();

    fn scan_dir(dir: &Path, acc: &mut Vec<std::path::PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    scan_dir(&path, acc);
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    acc.push(path);
                }
            }
        }
    }

    scan_dir(&crates_dir, &mut rs_files);

    for file_path in rs_files {
        let rel_path = file_path
            .strip_prefix(root_dir())
            .unwrap_or(&file_path)
            .to_string_lossy();

        if rel_path.starts_with("crates/xtask") {
            continue;
        }

        let content = fs::read_to_string(&file_path).expect("ler arquivo fonte");
        assert!(
            !content.contains("\"cut.session\""),
            "VIOLAÇÃO ARQUITETURAL (G5): {rel_path} ainda utiliza 'cut.session' em memória temporária de UI!"
        );
        assert!(
            !content.contains("\"modal.pointer\""),
            "VIOLAÇÃO ARQUITETURAL (G5): {rel_path} ainda utiliza 'modal.pointer' em memória temporária de UI!"
        );
    }
}

#[test]
fn app_state_decomposed_into_cohesive_substates() {
    let state_rs = root_dir().join("crates/core/src/state.rs");
    let content = fs::read_to_string(&state_rs).expect("crates/core/src/state.rs");

    // Valida que as 5 sub-estruturas coesas existem no módulo de estado
    assert!(
        content.contains("pub struct ProjectState"),
        "ProjectState deve existir"
    );
    assert!(
        content.contains("pub struct EditorSession"),
        "EditorSession deve existir"
    );
    assert!(
        content.contains("pub struct ToolState"),
        "ToolState deve existir"
    );
    assert!(
        content.contains("pub struct UiState"),
        "UiState deve existir"
    );
    assert!(
        content.contains("pub struct RenderResources"),
        "RenderResources deve existir"
    );

    // Valida que AppState é composto exclusivamente pelas sub-estruturas + EventBus
    assert!(
        content.contains("pub project: ProjectState"),
        "AppState deve conter project: ProjectState"
    );
    assert!(
        content.contains("pub session: EditorSession"),
        "AppState deve conter session: EditorSession"
    );
    assert!(
        content.contains("pub ui: UiState"),
        "AppState deve conter ui: UiState"
    );
    assert!(
        content.contains("pub render: RenderResources"),
        "AppState deve conter render: RenderResources"
    );
    assert!(
        content.contains("pub events: EventBus"),
        "AppState deve conter events: EventBus"
    );
    assert!(
        content.contains("pub tools: ToolState"),
        "EditorSession deve conter tools: ToolState"
    );

    // Valida isolamento de domínio: ProjectState, EditorSession e ToolState não contêm campos de UI/GPU
    let check_no_leaked_ui_gpu_fields = |struct_name: &str, forbidden: &[&str]| {
        let start = content
            .find(struct_name)
            .unwrap_or_else(|| panic!("struct {struct_name} not found"));
        let end = content[start..]
            .find('}')
            .unwrap_or_else(|| panic!("closing brace for {struct_name} not found"))
            + start;
        let body = &content[start..end];
        for f in forbidden {
            assert!(
                !body.contains(f),
                "VIOLAÇÃO ARQUITETURAL (G6 / F-002): {struct_name} contém campo '{f}' de apresentação/GPU vazado no domínio!"
            );
        }
    };

    let forbidden_in_domain = [
        "viewport_rect",
        "viewport_pixels_per_point",
        "backend_name",
        "stats",
        "show_settings",
        "show_help",
        "show_asset_library",
        "keybinds",
        "active_theme_id",
        "canvas_dirty",
    ];

    check_no_leaked_ui_gpu_fields("pub struct ProjectState {", &forbidden_in_domain);
    check_no_leaked_ui_gpu_fields("pub struct EditorSession {", &forbidden_in_domain);
    check_no_leaked_ui_gpu_fields("pub struct ToolState {", &forbidden_in_domain);
}

#[test]
fn module_crates_must_not_depend_on_egui() {
    let module_crates = [
        "crates/module-model",
        "crates/module-paint",
        "crates/module-uv",
        "crates/module-assets",
    ];

    for module in module_crates {
        let manifest_path = root_dir().join(module).join("Cargo.toml");
        let content = fs::read_to_string(&manifest_path)
            .unwrap_or_else(|_| panic!("Falha ao ler {}", manifest_path.display()));
        assert!(
            !content.contains("egui"),
            "VIOLAÇÃO ARQUITETURAL (G7 / F-008): {module}/Cargo.toml não pode depender de egui!"
        );
    }
}

#[test]
fn module_crates_sources_must_not_reference_egui() {
    let module_crates = [
        "crates/module-model",
        "crates/module-paint",
        "crates/module-uv",
        "crates/module-assets",
    ];

    let mut rs_files = Vec::new();
    fn scan_dir(dir: &Path, acc: &mut Vec<std::path::PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    scan_dir(&path, acc);
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    acc.push(path);
                }
            }
        }
    }

    for module in module_crates {
        let src_dir = root_dir().join(module).join("src");
        scan_dir(&src_dir, &mut rs_files);
    }

    for file_path in rs_files {
        let rel_path = file_path
            .strip_prefix(root_dir())
            .unwrap_or(&file_path)
            .to_string_lossy();
        let content = fs::read_to_string(&file_path).expect("ler arquivo fonte de módulo");
        assert!(
            !content.contains("egui::") && !content.contains("use egui"),
            "VIOLAÇÃO ARQUITETURAL (G7 / F-008): {rel_path} contém referência direta a egui!"
        );
    }
}

#[test]
fn petunia_cli_must_be_pure_headless() {
    let manifest_path = root_dir().join("crates/cli/Cargo.toml");
    let content = fs::read_to_string(&manifest_path).expect("crates/cli/Cargo.toml");
    assert!(
        !content.contains("egui"),
        "VIOLAÇÃO ARQUITETURAL (G8 / F-011): crates/cli não pode depender de egui!"
    );

    let src_path = root_dir().join("crates/cli/src/main.rs");
    let src_content = fs::read_to_string(&src_path).expect("crates/cli/src/main.rs");
    assert!(
        !src_content.contains("egui"),
        "VIOLAÇÃO ARQUITETURAL (G8 / F-011): crates/cli/src/main.rs não pode referenciar egui!"
    );
}

#[test]
fn application_api_dtos_and_stable_uuids_enforced() {
    let queries_rs = root_dir().join("crates/core/src/queries.rs");
    let content = fs::read_to_string(&queries_rs).expect("crates/core/src/queries.rs");

    assert!(content.contains("pub struct SceneHierarchyDto"));
    assert!(content.contains("pub struct SceneObjectDto"));
    assert!(content.contains("pub struct SelectionDetailsDto"));
    assert!(content.contains("pub struct ToolStatusDto"));

    let state_rs = root_dir().join("crates/core/src/state.rs");
    let state_content = fs::read_to_string(&state_rs).expect("crates/core/src/state.rs");
    assert!(
        state_content.contains("pub export_selected: Vec<Uuid>"),
        "export_selected deve armazenar Vec<Uuid> estável, e não usize instável"
    );

    let outliner_rs = root_dir().join("crates/ui/src/outliner.rs");
    let outliner_content = fs::read_to_string(&outliner_rs).expect("crates/ui/src/outliner.rs");
    assert!(
        outliner_content.contains("Asset(Uuid)"),
        "OutlinerNodeId::Asset deve referenciar Uuid estável"
    );
}

#[test]
fn petunia_ffi_must_be_pure_headless() {
    let manifest_path = root_dir().join("crates/ffi/Cargo.toml");
    let content = fs::read_to_string(&manifest_path).expect("crates/ffi/Cargo.toml");
    assert!(
        !content.contains("egui"),
        "VIOLAÇÃO ARQUITETURAL (G10): crates/ffi não pode depender de egui!"
    );

    let src_path = root_dir().join("crates/ffi/src/lib.rs");
    let src_content = fs::read_to_string(&src_path).expect("crates/ffi/src/lib.rs");
    assert!(
        !src_content.contains("egui"),
        "VIOLAÇÃO ARQUITETURAL (G10): crates/ffi/src/lib.rs não pode referenciar egui!"
    );
}

#[test]
fn tools_must_not_depend_on_physical_keycodes() {
    let tool_crates = [
        "crates/module-model",
        "crates/core/src/modal.rs",
        "crates/core/src/cutting_session.rs",
    ];

    let mut rs_files = Vec::new();
    fn scan_path(p: &Path, acc: &mut Vec<std::path::PathBuf>) {
        if p.is_dir() {
            if let Ok(entries) = fs::read_dir(p) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        scan_path(&path, acc);
                    } else if path.extension().is_some_and(|ext| ext == "rs") {
                        acc.push(path);
                    }
                }
            }
        } else if p.exists() {
            acc.push(p.to_path_buf());
        }
    }

    for target in tool_crates {
        let p = root_dir().join(target);
        scan_path(&p, &mut rs_files);
    }

    for file_path in rs_files {
        let rel_path = file_path
            .strip_prefix(root_dir())
            .unwrap_or(&file_path)
            .to_string_lossy();
        let content = fs::read_to_string(&file_path).expect("ler arquivo de ferramentas");
        assert!(
            !content.contains("winit::keyboard"),
            "VIOLAÇÃO ARQUITETURAL (Wave 1): {rel_path} referencia winit::keyboard! Ferramentas não podem conhecer atalhos físicos."
        );
        assert!(
            !content.contains("PhysicalKey"),
            "VIOLAÇÃO ARQUITETURAL (Wave 1): {rel_path} referencia PhysicalKey! Ferramentas não podem conhecer atalhos físicos."
        );
        assert!(
            !content.contains("egui::Key"),
            "VIOLAÇÃO ARQUITETURAL (Wave 1): {rel_path} referencia egui::Key! Ferramentas não podem conhecer atalhos físicos."
        );
    }
}

#[test]
fn core_and_domain_must_not_depend_on_eframe() {
    let domain_crates = [
        "crates/core",
        "crates/mesh",
        "crates/commands",
        "crates/config",
        "crates/project",
        "crates/render",
        "crates/module-model",
        "crates/module-paint",
        "crates/module-uv",
        "crates/module-assets",
        "crates/cli",
        "crates/ffi",
    ];

    for c in domain_crates {
        let cargo_path = root_dir().join(c).join("Cargo.toml");
        if cargo_path.exists() {
            let content = fs::read_to_string(&cargo_path).expect("ler Cargo.toml de domínio");
            assert!(
                !content.contains("eframe"),
                "VIOLAÇÃO ARQUITETURAL (Wave 1): {c}/Cargo.toml não pode depender de eframe!"
            );
        }
    }
}

#[test]
fn project_and_persistence_must_not_depend_on_egui() {
    let project_crates = [
        "crates/project",
        "crates/core/src/project_service.rs",
        "crates/core/src/recent_projects.rs",
    ];

    for p in project_crates {
        let p_path = root_dir().join(p);
        if p_path.is_file() {
            let content = fs::read_to_string(&p_path).expect("ler arquivo de projeto");
            assert!(
                !content.contains("egui::") && !content.contains("eframe"),
                "VIOLAÇÃO ARQUITETURAL (Wave 2): {p} referencia egui/eframe!"
            );
        } else if p_path.is_dir() {
            let src_dir = p_path.join("src");
            let target_dir = if src_dir.exists() { src_dir } else { p_path };
            for entry in fs::read_dir(&target_dir).expect("ler diretório") {
                let entry = entry.expect("entrada");
                if entry.path().extension().is_some_and(|e| e == "rs") {
                    let content = fs::read_to_string(entry.path()).expect("ler código");
                    assert!(
                        !content.contains("egui::") && !content.contains("eframe"),
                        "VIOLAÇÃO ARQUITETURAL (Wave 2): {} referencia egui/eframe!",
                        entry.path().display()
                    );
                }
            }
        }
    }
}

#[test]
fn petunia_core_must_not_depend_on_render() {
    let content = fs::read_to_string(root_dir().join("crates/core/Cargo.toml")).unwrap();
    assert!(
        !content.contains("petunia_render"),
        "VIOLAÇÃO: petunia_core não pode depender de petunia_render"
    );
}

#[test]
fn mcp_must_not_own_project_or_undo() {
    let content = fs::read_to_string(root_dir().join("crates/mcp/src/server.rs")).unwrap();
    assert!(
        !content.contains("UndoStack<Project>"),
        "VIOLAÇÃO: MCP não pode possuir UndoStack<Project>"
    );
    assert!(
        !content.contains("struct McpDomain"),
        "VIOLAÇÃO: MCP não pode possuir McpDomain"
    );
}

#[test]
fn cli_and_ffi_must_not_call_tool_apply() {
    let cli = fs::read_to_string(root_dir().join("crates/cli/src/main.rs")).unwrap();
    let ffi = fs::read_to_string(root_dir().join("crates/ffi/src/lib.rs")).unwrap();
    assert!(
        !cli.contains("ExtrudeTool::"),
        "CLI não pode chamar ExtrudeTool"
    );
    assert!(
        !ffi.contains("ExtrudeTool::"),
        "FFI não pode chamar ExtrudeTool"
    );
    assert!(
        !ffi.contains("PrimitivesTool::"),
        "FFI não pode chamar PrimitivesTool"
    );
}
