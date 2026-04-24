use std::error::Error;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ansi_term::{Color, Style};
use clap::Parser;
use emmylua_code_analysis::{
    EmmyLuaAnalysis, LuaDocument, WorkspaceFolder,
    collect_workspace_files, load_configs,
};
use emmy_lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Uri};
use tokio_util::sync::CancellationToken;
use url::Url;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    /// Configuration file path.
    /// If omitted, `.emmyrc.json` or `.luarc.json` in the file's parent directory is searched.
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Path to the file to check.
    file: PathBuf,

    /// Comma separated list of ignore patterns. Patterns must follow glob syntax.
    #[arg(short, long, value_delimiter = ',')]
    ignore: Option<Vec<String>>,

    /// Treat warnings as errors.
    #[arg(long)]
    warnings_as_errors: bool,

    /// Verbose output.
    #[arg(long)]
    verbose: bool,
}

fn resolve_path(path: &Path) -> Result<PathBuf, Box<dyn Error + Sync + Send>> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    Ok(absolute.canonicalize()?)
}

fn path_to_uri(path: &Path) -> Option<Uri> {
    let url = Url::from_file_path(path).ok()?;
    url.as_str().parse().ok()
}

fn supports_color() -> bool {
    std::io::stdout().is_terminal()
}

fn supports_underline() -> bool {
    supports_color()
        && (std::env::var("TERM").is_ok() || std::env::var("WT_SESSION").is_ok())
}

fn print_file_header(
    file_path: &str,
    error_count: usize,
    warning_count: usize,
    info_count: usize,
    hint_count: usize,
) {
    let color = supports_color();
    if color {
        print!("{}", Color::Cyan.bold().paint("--- "));
        print!("{}", Color::White.bold().paint(file_path));
        print!("{}", Color::Cyan.bold().paint(" "));
    } else {
        print!("--- {} ", file_path);
    }

    let mut parts = Vec::new();
    if error_count > 0 {
        let text = format!("{} error{}", error_count, if error_count > 1 { "s" } else { "" });
        parts.push(if color {
            Color::Red.bold().paint(text).to_string()
        } else {
            text
        });
    }
    if warning_count > 0 {
        let text = format!(
            "{} warning{}",
            warning_count,
            if warning_count > 1 { "s" } else { "" }
        );
        parts.push(if color {
            Color::Yellow.bold().paint(text).to_string()
        } else {
            text
        });
    }
    if info_count > 0 {
        let text = format!("{} info", info_count);
        parts.push(if color {
            Color::Purple.bold().paint(text).to_string()
        } else {
            text
        });
    }
    if hint_count > 0 {
        let text = format!("{} hint{}", hint_count, if hint_count > 1 { "s" } else { "" });
        parts.push(if color {
            Color::Cyan.bold().paint(text).to_string()
        } else {
            text
        });
    }

    if !parts.is_empty() {
        print!("[{}]", parts.join(", "));
    }
    println!();
}

fn display_single_diagnostic(
    file_path: &str,
    document: &LuaDocument,
    lines: &[&str],
    diagnostic: Diagnostic,
) {
    let range = diagnostic.range;
    let color = supports_color();
    let underline = supports_underline();

    let (level_color, level_symbol) = match diagnostic.severity {
        Some(DiagnosticSeverity::ERROR) => (Color::Red, "error"),
        Some(DiagnosticSeverity::WARNING) => (Color::Yellow, "warning"),
        Some(DiagnosticSeverity::INFORMATION) => (Color::Purple, "info"),
        Some(DiagnosticSeverity::HINT) => (Color::Cyan, "hint"),
        _ => (Color::Red, "error"),
    };

    let code = diagnostic.code.as_ref().map(|c| match c {
        NumberOrString::Number(n) => format!("[{}]", n),
        NumberOrString::String(s) => format!("[{}]", s),
    }).unwrap_or_default();

    let start_line = range.start.line as usize;
    let start_character = range.start.character as usize;
    let start_col = document
        .get_col_offset_at_line(start_line, start_character)
        .map(|c| u32::from(c) as usize)
        .unwrap_or(start_character);
    let end_line = range.end.line as usize;
    let end_character = range.end.character as usize;
    let end_col = document
        .get_col_offset_at_line(end_line, end_character)
        .map(|c| u32::from(c) as usize)
        .unwrap_or(end_character);

    if start_line >= lines.len() {
        return;
    }

    // Print diagnostic header
    if color {
        print!("{}: ", level_color.bold().paint(level_symbol));
        print!("{}", Style::new().bold().paint(&diagnostic.message));
        if !code.is_empty() {
            print!(" {}", Color::Fixed(8).paint(&code));
        }
    } else {
        print!("{}: {}", level_symbol, diagnostic.message);
        if !code.is_empty() {
            print!(" {}", code);
        }
    }
    println!();

    // Print location
    if color {
        println!(
            "  {}: {}:{}:{}",
            Color::Fixed(8).paint("-->"),
            file_path,
            start_line + 1,
            start_character + 1
        );
    } else {
        println!("  --> {}:{}:{}", file_path, start_line + 1, start_character + 1);
    }
    println!();

    // Context lines
    let context_start = start_line.saturating_sub(1);
    let context_end = (end_line + 1).min(lines.len().saturating_sub(1));
    let line_num_width = (context_end + 1).to_string().len();

    for (i, line_text) in lines.iter().enumerate().take(context_end + 1).skip(context_start) {
        let line_num = i + 1;
        if color {
            print!(
                "  {} │ ",
                Color::Cyan.paint(format!("{:width$}", line_num, width = line_num_width))
            );
        } else {
            print!("  {:width$} | ", line_num, width = line_num_width);
        }

        if i >= start_line && i <= end_line {
            if i == start_line && i == end_line {
                // Single line
                let prefix = &line_text[..std::cmp::min(start_col, line_text.len())];
                let error_part = if start_col < line_text.len() && end_col <= line_text.len() {
                    &line_text[start_col..end_col]
                } else if start_col < line_text.len() {
                    &line_text[start_col..]
                } else {
                    ""
                };
                let suffix = if end_col < line_text.len() {
                    &line_text[end_col..]
                } else {
                    ""
                };

                print!("{}", prefix);
                if color && !error_part.is_empty() {
                    let mut style = level_color.bold();
                    if underline {
                        style = style.underline();
                    }
                    print!("{}", style.paint(error_part));
                } else {
                    print!("{}", error_part);
                }
                println!("{}", suffix);
            } else {
                // Multi-line
                if color {
                    let mut style = level_color.bold();
                    if underline {
                        style = style.underline();
                    }
                    println!("{}", style.paint(*line_text));
                } else {
                    println!("{}", line_text);
                }
            }
        } else {
            println!("{}", line_text);
        }
    }
    println!();
}

fn print_summary(
    total_errors: usize,
    total_warnings: usize,
    total_info: usize,
    total_hints: usize,
    warnings_as_errors: bool,
) -> i32 {
    let color = supports_color();

    if total_errors == 0 && total_warnings == 0 && total_info == 0 && total_hints == 0 {
        println!();
        if color {
            println!("{}", Color::Cyan.bold().paint("Summary"));
        } else {
            println!("Summary");
        }
        println!("  No issues found");
        println!();
        if color {
            println!("{}", Color::Green.bold().paint("Check successful"));
        } else {
            println!("Check successful");
        }
        return 0;
    }

    println!();
    if color {
        println!("{}", Color::Cyan.bold().paint("Summary"));
    } else {
        println!("Summary");
    }

    if total_errors > 0 {
        let text = format!(
            "  {} error{}",
            total_errors,
            if total_errors > 1 { "s" } else { "" }
        );
        if color {
            println!("{}", Color::Red.bold().paint(text));
        } else {
            println!("{}", text);
        }
    }
    if total_warnings > 0 {
        let text = format!(
            "  {} warning{}",
            total_warnings,
            if total_warnings > 1 { "s" } else { "" }
        );
        if color {
            println!("{}", Color::Yellow.bold().paint(text));
        } else {
            println!("{}", text);
        }
    }
    if total_info > 0 {
        let text = format!("  {} info", total_info);
        if color {
            println!("{}", Color::Purple.bold().paint(text));
        } else {
            println!("{}", text);
        }
    }
    if total_hints > 0 {
        let text = format!(
            "  {} hint{}",
            total_hints,
            if total_hints > 1 { "s" } else { "" }
        );
        if color {
            println!("{}", Color::Cyan.bold().paint(text));
        } else {
            println!("{}", text);
        }
    }

    let has_error = total_errors > 0;
    let has_warning = total_warnings > 0;

    if has_error || (warnings_as_errors && has_warning) {
        println!();
        if color {
            println!("{}", Color::Red.bold().paint("Check failed"));
        } else {
            println!("Check failed");
        }
        1
    } else if has_warning {
        println!();
        if color {
            println!(
                "{}",
                Color::Yellow.bold().paint("Check completed with warnings")
            );
        } else {
            println!("Check completed with warnings");
        }
        0
    } else {
        println!();
        if color {
            println!("{}", Color::Green.bold().paint("Check successful"));
        } else {
            println!("Check successful");
        }
        0
    }
}

fn find_config(file: &Path) -> Result<PathBuf, Box<dyn Error + Sync + Send>> {
    let mut dir = if file.is_dir() {
        file
    } else {
        file.parent().ok_or("file has no parent directory")?
    };
    loop {
        for name in [".emmyrc.json", ".luarc.json"] {
            let candidate = dir.join(name);
            if candidate.exists() {
                return Ok(candidate.canonicalize()?);
            }
        }
        dir = match dir.parent() {
            Some(p) => p,
            None => break,
        };
    }
    Err("no .emmyrc.json or .luarc.json found in file's directory or any ancestor".into())
}

/// Collect all `.lua` files recursively under the given directory.
fn collect_lua_files(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn Error + Sync + Send>> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext.eq_ignore_ascii_case("lua") {
                    files.push(path.to_path_buf());
                }
            }
        }
    }
    // Sort for deterministic output.
    files.sort();
    Ok(files)
}

struct FileDiagnostics {
    file_path: String,
    error_count: usize,
    warning_count: usize,
    info_count: usize,
    hint_count: usize,
    diagnostics: Vec<Diagnostic>,
}

fn diagnose_single_file(
    analysis: &EmmyLuaAnalysis,
    config_root: &Path,
    file: &Path,
) -> Option<FileDiagnostics> {
    let target_uri = path_to_uri(file)?;
    let file_id = analysis.get_file_id(&target_uri)?;

    let cancel = CancellationToken::new();
    let diagnostics = analysis.diagnose_file(file_id, cancel);

    let db = analysis.compilation.get_db();
    let file_path = db
        .get_vfs()
        .get_file_path(&file_id)
        .and_then(|p| p.strip_prefix(config_root).ok())
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| {
            file.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string()
        });

    let mut error_count = 0;
    let mut warning_count = 0;
    let mut info_count = 0;
    let mut hint_count = 0;

    let diagnostics = diagnostics.unwrap_or_default();
    for diag in &diagnostics {
        match diag.severity {
            Some(DiagnosticSeverity::ERROR) => error_count += 1,
            Some(DiagnosticSeverity::WARNING) => warning_count += 1,
            Some(DiagnosticSeverity::INFORMATION) => info_count += 1,
            Some(DiagnosticSeverity::HINT) => hint_count += 1,
            _ => error_count += 1,
        }
    }

    Some(FileDiagnostics {
        file_path,
        error_count,
        warning_count,
        info_count,
        hint_count,
        diagnostics,
    })
}

fn run_check(
    analysis: &EmmyLuaAnalysis,
    config_root: &Path,
    target_files: &[PathBuf],
    warnings_as_errors: bool,
) -> i32 {
    let mut total_errors = 0usize;
    let mut total_warnings = 0usize;
    let mut total_info = 0usize;
    let mut total_hints = 0usize;

    let db = analysis.compilation.get_db();

    for file in target_files {
        let Some(result) = diagnose_single_file(analysis, config_root, file) else {
            continue;
        };

        total_errors += result.error_count;
        total_warnings += result.warning_count;
        total_info += result.info_count;
        total_hints += result.hint_count;

        print_file_header(
            &result.file_path,
            result.error_count,
            result.warning_count,
            result.info_count,
            result.hint_count,
        );
        println!();

        if !result.diagnostics.is_empty() {
            let target_uri = path_to_uri(file).unwrap();
            let file_id = analysis.get_file_id(&target_uri).unwrap();
            let document = db.get_vfs().get_document(&file_id).unwrap();
            let text = document.get_text();
            let lines: Vec<&str> = text.lines().collect();

            for diagnostic in result.diagnostics {
                display_single_diagnostic(&result.file_path, &document, &lines, diagnostic);
            }
        }
    }

    print_summary(
        total_errors,
        total_warnings,
        total_info,
        total_hints,
        warnings_as_errors,
    )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    let args = Args::parse();
    let input = resolve_path(&args.file)?;

    let config = match args.config {
        Some(c) => resolve_path(&c)?,
        None => find_config(&input)?,
    };

    let config_root = config.parent().ok_or("config has no parent")?.to_path_buf();

    // Determine target files: single file or all .lua files under a directory.
    let target_files: Vec<PathBuf>;
    let is_dir = input.is_dir();
    if is_dir {
        if !input.starts_with(&config_root) {
            return Err(format!(
                "directory {} is outside config root {}",
                input.display(),
                config_root.display()
            )
            .into());
        }
        target_files = collect_lua_files(&input)?;
        if target_files.is_empty() {
            eprintln!("No .lua files found in {}", input.display());
            return Ok(());
        }
    } else {
        if !input.starts_with(&config_root) {
            return Err(format!(
                "file {} is outside config root {}",
                input.display(),
                config_root.display()
            )
            .into());
        }
        target_files = vec![input.clone()];
    }

    // 1. Load and process emmyrc
    let mut emmyrc = load_configs(vec![config], None);
    emmyrc.pre_process_emmyrc(&config_root);

    // 2. Create analysis
    let mut analysis = EmmyLuaAnalysis::new();
    analysis.update_config(Arc::new(emmyrc.clone()));
    analysis.init_std_lib(None);

    // 3. Set up workspaces
    analysis.add_main_workspace(config_root.clone());
    let mut workspace_folders = vec![WorkspaceFolder::new(config_root.clone(), false)];
    for lib in &emmyrc.workspace.library {
        let path = PathBuf::from(lib.get_path());
        if path.exists() {
            analysis.add_library_workspace(path.clone());
            workspace_folders.push(WorkspaceFolder::new(path, true));
        }
    }
    for folder in &workspace_folders {
        analysis.add_main_workspace(folder.root.clone());
    }

    // 4. Collect and load files
    let file_infos = collect_workspace_files(&workspace_folders, &emmyrc, None, args.ignore);
    let files: Vec<_> = file_infos
        .into_iter()
        .filter(|f| !f.path.ends_with(".editorconfig"))
        .map(|f| f.into_tuple())
        .collect();
    analysis.update_files_by_path(files);

    // 5. Diagnose target files and output
    let exit_code = run_check(
        &analysis,
        &config_root,
        &target_files,
        args.warnings_as_errors,
    );

    eprintln!("Check finished");

    if exit_code != 0 {
        std::process::exit(exit_code);
    }

    Ok(())
}
