//! `cargo xtask ui-guard` — guarda de arquitetura da camada de UI.
//!
//! Diretriz normativa: `PETUNIA3D_EGUI_ECOSYSTEM_FINAL_PUSH_DIRECTIVE.md` §36
//! (proibições de código) e §37 (`ui-guard` obrigatório).
//!
//! Filosofia que este guard protege:
//!
//! > **Raw egui para composição simples. Bibliotecas especializadas para
//! > problemas especializados. Petunia Components e Adapters como única
//! > superfície permitida para product UI.**
//!
//! Fases previstas pela diretriz:
//!
//! | Fase | Comportamento |
//! | --- | --- |
//! | Wave 1 | report (este modo) |
//! | Wave 2 | warning na CI |
//! | Wave 3 | erro para os arquivos já migrados |
//!
//! Por isso o guard **não falha por padrão**: ele mede e reporta. `--strict`
//! liga o comportamento de erro para as regras de confinamento de adapter, que
//! já valem hoje (nenhum tipo de biblioteca auxiliar pode escapar do adapter).

use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Severidade de uma regra.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuleKind {
    /// O padrão só pode existir em foundation/adapter. Vale já hoje.
    AdapterOnly,
    /// Cheiro de produto: layout responsivo feito à mão. Migração progressiva.
    ProductSmell,
    /// Informativo: mede a migração, não acusa.
    Advisory,
}

struct Rule {
    id: &'static str,
    pattern: &'static str,
    kind: RuleKind,
    /// Solução normativa segundo a matriz obrigatória (§17.2).
    replacement: &'static str,
    /// Palavra-chave que libera o arquivo para esta regra (além de
    /// [`FOUNDATION_PATHS`]). Vazio = só foundation libera.
    allow_path_contains: &'static [&'static str],
}

/// Prefixos de diretório tratados como foundation/adapter (§22 da diretiva).
/// Todo arquivo aqui pode conhecer detalhes das crates auxiliares.
const FOUNDATION_DIRS: &[&str] = &[
    "crates/ui/src/adapters/",
    "crates/ui/src/foundation/",
    "crates/ui/src/components/",
];

/// Caminhos de implementação de componente de baixo nível, de superfície de
/// adapter ainda não dobrada em `adapters/`, e das exceções legítimas listadas
/// em §36 (viewport, canvas UV, régua da timeline, gizmo, visualização de dados,
/// componente de baixo nível).
const FOUNDATION_PATHS: &[&str] = &[
    // Componentes de baixo nível e tokens.
    "crates/ui/src/widgets.rs",
    "crates/ui/src/inspector_widgets.rs",
    "crates/ui/src/tokens.rs",
    "crates/ui/src/regions.rs",
    "crates/ui/src/icon_registry.rs",
    "crates/ui/src/icons.rs",
    // Superfícies de adapter cuja implementação ainda vive fora de `adapters/`.
    "crates/ui/src/file_dialog_service.rs",
    "crates/ui/src/transform_gizmo_integration.rs",
    "crates/ui/src/icon_provider.rs",
    // Exceções de §36: estas áreas podem calcular geometria própria.
    "crates/ui/src/nav_gizmo.rs",
    "crates/ui/src/timeline.rs",
    "crates/ui/src/modules_ui/uv_ui.rs",
    "crates/ui/src/modules_ui/paint_ui.rs",
    "crates/ui/src/modules_ui/animation_ui.rs",
    // Dev tooling e testes.
    "crates/ui/src/devtools.rs",
    "crates/ui/src/paint_tests.rs",
    "crates/ui/src/modal_tests.rs",
    "crates/ui/src/cutting_tests.rs",
    // Vitrine de componentes (§38): é a superfície onde um componente
    // reutilizável nasce e é comparado entre temas/densidades/escalas. Por
    // definição ela monta micro-layout e mede o resultado — o que o product
    // code continua proibido de fazer. Não é caminho de produto: nenhum painel
    // do shell importa este módulo.
    "crates/ui/src/gallery.rs",
];

/// Regras da §37, na ordem em que aparecem na diretriz.
const RULES: &[Rule] = &[
    Rule {
        id: "egui_taffy",
        pattern: "egui_taffy::",
        kind: RuleKind::AdapterOnly,
        replacement: "PetuniaResponsiveLayout (crate::adapters::taffy_layout)",
        allow_path_contains: &["adapters/taffy_layout.rs"],
    },
    Rule {
        id: "egui_tiles",
        pattern: "egui_tiles::",
        kind: RuleKind::AdapterOnly,
        replacement: "PetuniaLayoutAdapter (crate::adapters::tile_layout)",
        allow_path_contains: &["adapters/tile_layout.rs"],
    },
    Rule {
        id: "egui_dnd",
        pattern: "egui_dnd::",
        kind: RuleKind::AdapterOnly,
        replacement: "PetuniaDragList (crate::adapters::drag_drop)",
        allow_path_contains: &["adapters/drag_drop.rs"],
    },
    Rule {
        id: "egui_animation",
        pattern: "egui_animation::",
        kind: RuleKind::AdapterOnly,
        replacement: "PetuniaMotion (crate::foundation::motion)",
        allow_path_contains: &["foundation/motion.rs"],
    },
    Rule {
        id: "egui_form",
        pattern: "egui_form::",
        kind: RuleKind::AdapterOnly,
        replacement: "PetuniaFormSession (crate::adapters::form)",
        allow_path_contains: &["adapters/form.rs"],
    },
    Rule {
        id: "twill",
        pattern: "twill::",
        kind: RuleKind::AdapterOnly,
        replacement: "Petunia tokens (crate::foundation::theme) / Twill adapter",
        allow_path_contains: &["adapters/twill_tokens.rs"],
    },
    Rule {
        id: "egui_ltreeview",
        pattern: "egui_ltreeview::",
        kind: RuleKind::AdapterOnly,
        replacement: "crate::adapters::tree (façade da árvore Parts/Scene)",
        allow_path_contains: &["adapters/tree.rs"],
    },
    Rule {
        id: "iconflow",
        pattern: "iconflow::",
        kind: RuleKind::AdapterOnly,
        replacement: "crate::adapters::icons (IconRegistry / PetuniaIcon)",
        allow_path_contains: &["adapters/icons.rs", "icon_provider.rs"],
    },
    Rule {
        id: "egui_inbox",
        pattern: "egui_inbox::",
        kind: RuleKind::AdapterOnly,
        replacement: "crate::adapters::inbox (UiBridge)",
        allow_path_contains: &["adapters/inbox.rs"],
    },
    Rule {
        id: "egui_file_dialog",
        pattern: "egui_file_dialog::",
        kind: RuleKind::AdapterOnly,
        replacement: "crate::adapters::file_dialog (PetuniaFileDialogService)",
        allow_path_contains: &["file_dialog_service.rs"],
    },
    Rule {
        id: "transform_gizmo",
        // `transform_gizmo::` (a crate) e não `transform_gizmo_integration`
        // (o módulo Petunia), que é declaração legítima em `lib.rs`.
        pattern: "transform_gizmo::",
        kind: RuleKind::AdapterOnly,
        replacement: "crate::adapters::gizmo (PetuniaTransformGizmo)",
        allow_path_contains: &["transform_gizmo_integration.rs"],
    },
    Rule {
        id: "available_width",
        pattern: "available_width()",
        kind: RuleKind::ProductSmell,
        replacement: "PetuniaTaffyLayout (layout complexo/responsivo)",
        allow_path_contains: &[],
    },
    Rule {
        id: "spacing_mut",
        pattern: "spacing_mut()",
        kind: RuleKind::ProductSmell,
        replacement: "tokens de spacing/radius/density centralizados",
        allow_path_contains: &[],
    },
    Rule {
        id: "allocate_exact_size",
        pattern: "allocate_exact_size",
        kind: RuleKind::ProductSmell,
        replacement: "Petunia Component existente (ou implementação de baixo nível)",
        allow_path_contains: &[],
    },
    Rule {
        id: "painter_rect_filled",
        pattern: "painter().rect_filled",
        kind: RuleKind::ProductSmell,
        replacement: "Petunia Component / Painter de foundation",
        allow_path_contains: &[],
    },
    Rule {
        id: "egui_window",
        pattern: "egui::Window",
        kind: RuleKind::ProductSmell,
        replacement: "Petunia modal/popup adapter",
        allow_path_contains: &[
            "/settings_modal.rs",
            "/reference_manager.rs",
            "/tool_properties_popover.rs",
            "/primitive_card.rs",
            "/command_palette.rs",
            "/devtools.rs",
        ],
    },
    Rule {
        id: "egui_popup",
        pattern: "egui::Popup",
        kind: RuleKind::ProductSmell,
        replacement: "Petunia menu/popup adapter",
        allow_path_contains: &["/widgets.rs", "/toolbar.rs", "/outliner.rs"],
    },
    Rule {
        id: "ui_horizontal",
        pattern: "ui.horizontal",
        kind: RuleKind::Advisory,
        replacement: "egui built-in quando for micro row simples (permitido)",
        allow_path_contains: &[],
    },
    Rule {
        id: "color32_from",
        pattern: "Color32::from_",
        kind: RuleKind::Advisory,
        replacement: "tokens semânticos em vez de cor literal",
        allow_path_contains: &[],
    },
];

/// Uma ocorrência de regra em um arquivo.
struct Hit {
    rel: String,
    count: usize,
    foundation: bool,
}

/// Resultado consolidado de uma execução.
pub struct GuardReport {
    per_rule: Vec<(String, RuleKind, Vec<Hit>)>,
    rules_scanned: usize,
    files_scanned: usize,
}

impl GuardReport {
    /// Total de ocorrências fora de foundation/adapter (candidatos a correção).
    pub fn offenders(&self) -> usize {
        self.per_rule
            .iter()
            .flat_map(|(_, _, hits)| hits.iter())
            .filter(|h| !h.foundation)
            .map(|h| h.count)
            .sum()
    }

    /// Total de ocorrências em foundation/adapter (dentro do permitido).
    pub fn foundation_hits(&self) -> usize {
        self.per_rule
            .iter()
            .flat_map(|(_, _, hits)| hits.iter())
            .filter(|h| h.foundation)
            .map(|h| h.count)
            .sum()
    }
}

fn is_foundation(rel: &str) -> bool {
    FOUNDATION_PATHS.contains(&rel) || FOUNDATION_DIRS.iter().any(|p| rel.starts_with(p))
}

/// Coleta arquivos `.rs` de `crates/`, ignorando artefatos de build.
///
/// `crates/xtask` fica de fora: o próprio guard (e o `arch-check`) contém os
/// nomes das bibliotecas como dado, não como uso de product code.
fn rust_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let crates = root.join("crates");
    let mut stack = vec![crates.clone()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path
                    .file_name()
                    .is_some_and(|n| n == "target" || n == "xtask")
                {
                    continue;
                }
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

fn count_occurrences(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    haystack.matches(needle).count()
}

/// Executa a varredura do guard.
///
/// * `strict` — além de reportar, falha quando uma regra [`RuleKind::AdapterOnly`]
///   aparece fora de foundation/adapter.
pub fn run(root: &Path, strict: bool) -> Result<GuardReport> {
    println!("🧱 ui-guard — guarda de arquitetura da camada de UI");
    println!("    base: §36/§37 da Egui Ecosystem Final Push Directive\n");

    let files = rust_files(root)?;
    let mut per_rule: Vec<(String, RuleKind, Vec<Hit>)> = Vec::new();
    let mut strict_violations: Vec<String> = Vec::new();

    for rule in RULES {
        let mut hits: Vec<Hit> = Vec::new();
        for file in &files {
            let rel = file
                .strip_prefix(root)
                .unwrap_or(file)
                .to_string_lossy()
                .replace('\\', "/");
            let content =
                std::fs::read_to_string(file).with_context(|| format!("falha ao ler {rel}"))?;
            let count = count_occurrences(&content, rule.pattern);
            if count == 0 {
                continue;
            }
            let foundation =
                is_foundation(&rel) || rule.allow_path_contains.iter().any(|p| rel.contains(p));
            if !foundation && rule.kind == RuleKind::AdapterOnly {
                strict_violations.push(format!(
                    "{} usa `{}` fora de adapter (solução: {})",
                    rel, rule.pattern, rule.replacement
                ));
            }
            hits.push(Hit {
                rel,
                count,
                foundation,
            });
        }
        hits.sort_by(|a, b| b.count.cmp(&a.count).then(a.rel.cmp(&b.rel)));
        per_rule.push((rule.id.to_string(), rule.kind, hits));
    }

    let report = GuardReport {
        per_rule,
        rules_scanned: RULES.len(),
        files_scanned: files.len(),
    };

    print_report(&report);

    if strict && !strict_violations.is_empty() {
        for v in &strict_violations {
            eprintln!("❌ ui-guard (strict): {v}");
        }
        anyhow::bail!(
            "ui-guard (strict): {} violação(ões) de confinamento de adapter",
            strict_violations.len()
        );
    }

    Ok(report)
}

fn kind_label(kind: RuleKind) -> &'static str {
    match kind {
        RuleKind::AdapterOnly => "confinamento",
        RuleKind::ProductSmell => "cheiro de produto",
        RuleKind::Advisory => "informativo",
    }
}

fn print_report(report: &GuardReport) {
    println!(
        "📊 {} arquivos .rs varridos · {} regras\n",
        report.files_scanned, report.rules_scanned
    );
    println!(
        "{:<22} {:<18} {:>7} {:>12} {:>10}",
        "REGRA", "TIPO", "TOTAL", "FOUNDATION", "PRODUTO"
    );
    println!("{}", "-".repeat(72));

    for (id, kind, hits) in &report.per_rule {
        let total: usize = hits.iter().map(|h| h.count).sum();
        let foundation: usize = hits.iter().filter(|h| h.foundation).map(|h| h.count).sum();
        let product = total - foundation;
        println!(
            "{:<22} {:<18} {:>7} {:>12} {:>10}",
            id,
            kind_label(*kind),
            total,
            foundation,
            product
        );
    }
    println!("{}", "-".repeat(72));
    println!(
        "TOTAL fora de foundation/adapter (candidatos a migração): {}",
        report.offenders()
    );
    println!(
        "TOTAL dentro de foundation/adapter (permitido):          {}\n",
        report.foundation_hits()
    );

    println!("📍 Top arquivos de produto por regra:");
    for (id, kind, hits) in &report.per_rule {
        if *kind == RuleKind::Advisory {
            continue;
        }
        let product: Vec<&Hit> = hits.iter().filter(|h| !h.foundation).take(5).collect();
        if product.is_empty() {
            continue;
        }
        println!("  {id}:");
        for h in product {
            println!("    {:>3}  {}", h.count, h.rel);
        }
    }
    println!();
}

/// Emite a baseline em Markdown para `docs/audits/ui-ecosystem-final-push/00-baseline.md`.
pub fn markdown(report: &GuardReport) -> String {
    let mut out = String::new();
    out.push_str("<!-- Gerado por `cargo xtask ui-guard --baseline`. Não editar à mão. -->\n\n");
    out.push_str("| Regra | Tipo | Total | Foundation/adapter | Produto |\n");
    out.push_str("| :--- | :--- | ---: | ---: | ---: |\n");
    for (id, kind, hits) in &report.per_rule {
        let total: usize = hits.iter().map(|h| h.count).sum();
        let foundation: usize = hits.iter().filter(|h| h.foundation).map(|h| h.count).sum();
        out.push_str(&format!(
            "| `{id}` | {} | {total} | {foundation} | {} |\n",
            kind_label(*kind),
            total - foundation
        ));
    }
    out.push_str(&format!(
        "\nArquivos `.rs` varridos: **{}** · regras: **{}**\n",
        report.files_scanned, report.rules_scanned
    ));

    let mut by_file: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, kind, hits) in &report.per_rule {
        if *kind == RuleKind::Advisory {
            continue;
        }
        for h in hits.iter().filter(|h| !h.foundation) {
            *by_file.entry(h.rel.as_str()).or_default() += h.count;
        }
    }
    let mut ranked: Vec<(&str, usize)> = by_file.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));

    out.push_str("\n### Arquivos de produto mais afetados\n\n");
    out.push_str("| # | Arquivo | Ocorrências |\n| ---: | :--- | ---: |\n");
    for (i, (rel, count)) in ranked.iter().take(20).enumerate() {
        out.push_str(&format!("| {} | `{rel}` | {count} |\n", i + 1));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_needle_occurrences() {
        assert_eq!(count_occurrences("a.b() a.b()", "a.b()"), 2);
        assert_eq!(count_occurrences("", "x"), 0);
        assert_eq!(count_occurrences("x", ""), 0);
    }

    #[test]
    fn foundation_paths_are_recognised() {
        assert!(is_foundation("crates/ui/src/adapters/taffy_layout.rs"));
        assert!(is_foundation("crates/ui/src/foundation/spacing.rs"));
        assert!(is_foundation("crates/ui/src/widgets.rs"));
        assert!(!is_foundation("crates/ui/src/toolbar.rs"));
        assert!(!is_foundation("crates/ui/src/properties_panel.rs"));
    }

    #[test]
    fn every_rule_has_a_normative_replacement() {
        for rule in RULES {
            assert!(
                !rule.replacement.is_empty(),
                "regra {} precisa apontar a solução normativa (§17.2)",
                rule.id
            );
        }
    }

    #[test]
    fn run_is_deterministic_over_workspace() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("raiz do workspace")
            .to_path_buf();
        let first = run(&root, false).expect("primeira varredura");
        let second = run(&root, false).expect("segunda varredura");
        assert_eq!(first.offenders(), second.offenders());
        assert_eq!(first.files_scanned, second.files_scanned);
    }
}
