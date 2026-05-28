use crate::app::App;
use crate::models::{AppMode, InputPurpose, RightPanelView};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

fn format_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    if size == 0 {
        return "0 B".to_string();
    }
    let exp = (size as f64).log(1024.0).min(UNITS.len() as f64 - 1.0) as usize;
    let value = size as f64 / 1024f64.powi(exp as i32);
    format!("{:.1} {}", value, UNITS[exp])
}

fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let alloc_width = std::cmp::min(width, r.width);
    let alloc_height = std::cmp::min(height, r.height);

    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height.saturating_sub(alloc_height)) / 2),
            Constraint::Length(alloc_height),
            Constraint::Length((r.height.saturating_sub(alloc_height)) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((r.width.saturating_sub(alloc_width)) / 2),
            Constraint::Length(alloc_width),
            Constraint::Length((r.width.saturating_sub(alloc_width)) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Main Layout Split
            Constraint::Length(4), // Footer (Increased to fit 2 lines of text)
        ])
        .split(frame.size());

    render_header(frame, app, chunks[0]);
    render_main(frame, app, chunks[1]);
    render_footer(frame, app, chunks[2]);

    match app.mode {
        AppMode::UtilityMenu => render_utility_menu(frame, app),
        AppMode::FilterInput => render_input_modal(frame, app),
        AppMode::DryRunPreview => render_dry_run_modal(frame, app),
        _ => {}
    }
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let header_text = format!(" Working Directory: {} ", app.current_dir.display());
    let header = Paragraph::new(header_text).block(
        Block::default()
            .title(" TUI File Manager Environment ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(header, area);
}

fn render_main(frame: &mut Frame, app: &mut App, area: Rect) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(55), // Left: Slidably Scrolled File System Vector
            Constraint::Percentage(45), // Right: Multi-View Detail Analysis Module
        ])
        .split(area);

    if app.mode == AppMode::Settings {
        render_settings(frame, app, area);
    } else {
        // --- VIEWPORT SCROLL CALCULATIONS ---
        let border_padding = 2;
        let visible_height = (main_chunks[0].height as usize).saturating_sub(border_padding);

        // Push current window boundary layout size back to App core tracking logic
        app.adjust_viewport(visible_height);

        let end_idx = std::cmp::min(app.scroll_offset + visible_height, app.filtered_files.len());
        let visible_slice = if app.scroll_offset < end_idx {
            &app.filtered_files[app.scroll_offset..end_idx]
        } else {
            &[]
        };

        let items: Vec<ListItem> = visible_slice
            .iter()
            .enumerate()
            .map(|(i, file)| {
                let absolute_index = app.scroll_offset + i;
                let prefix = if file.is_dir { "📁 " } else { "📄 " };
                let text = format!("{}{:<35} {:>10}", prefix, file.name, format_size(file.size));

                let style = if absolute_index == app.selected_index {
                    Style::default()
                        .bg(Color::Blue)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(text).style(style)
            })
            .collect();

        let list_title = format!(
            " File Stream Vector Layout ({} items) ",
            app.filtered_files.len()
        );
        let file_list = List::new(items).block(
            Block::default()
                .title(list_title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        );
        frame.render_widget(file_list, main_chunks[0]);

        // Render Context Matching Component Matrix on Right View Panel
        match app.right_panel_view {
            RightPanelView::FileMetadata => {
                if let Some(file) = app.filtered_files.get(app.selected_index) {
                    let meta_text = format!(
                        "\n Name: {}\n Path: {}\n Size: {} bytes ({})\n Type: {}\n Permissions: {}\n Modified Timestamp: {}",
                        file.name,
                        file.path.display(),
                        file.size,
                        format_size(file.size),
                        if file.is_dir { "Directory Cluster" } else { "Data Node File" },
                        file.permissions,
                        file.modified
                    );
                    let meta_paragraph = Paragraph::new(meta_text)
                        .block(
                            Block::default()
                                .title(" Selection Metadata Inspection Properties ")
                                .borders(Borders::ALL)
                                .border_style(Style::default().fg(Color::Magenta)),
                        )
                        .wrap(Wrap { trim: true });
                    frame.render_widget(meta_paragraph, main_chunks[1]);
                } else {
                    let empty_block = Block::default()
                        .title(" Selection Metadata Inspection Properties ")
                        .borders(Borders::ALL);
                    frame.render_widget(empty_block, main_chunks[1]);
                }
            }
            RightPanelView::LargeFiles => {
                let items: Vec<ListItem> = app
                    .large_files
                    .iter()
                    .map(|(p, s)| {
                        ListItem::new(format!(
                            " [{}] -> {}",
                            format_size(*s),
                            p.file_name().unwrap_or_default().to_string_lossy()
                        ))
                    })
                    .collect();
                let list = List::new(items).block(
                    Block::default()
                        .title(" Target Top 20 Heavy Storage Assets Array ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Red)),
                );
                frame.render_widget(list, main_chunks[1]);
            }
            RightPanelView::Duplicates => {
                let mut flat_items = Vec::new();
                for (group_idx, group) in app.duplicates.iter().enumerate() {
                    flat_items.push(ListItem::new(format!(
                        "── Signature Verification Duplicate Group #{} ──",
                        group_idx + 1
                    )));
                    for path in group {
                        flat_items.push(ListItem::new(format!("   {}", path.display())));
                    }
                }
                let list = List::new(flat_items).block(
                    Block::default()
                        .title(" Duplicated Checksum Clones Identity Map ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::LightRed)),
                );
                frame.render_widget(list, main_chunks[1]);
            }
        }
    }
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let status_msg = if app.message.is_empty() {
        "Idle - Waiting for input..."
    } else {
        &app.message
    };

    // Dynamically swap the guide text based on the current active window
    let guide = match app.mode {
        AppMode::Settings => {
            "[j/k] Nav | [e/Enter] Edit Path | [x] Edit Exts | [n] New Cat | [d] Delete | [s] Save"
        }
        AppMode::FilterInput => "[Enter] Confirm | [Esc] Cancel",
        AppMode::DryRunPreview => "[y] Confirm Changes | [n/Esc] Cancel",
        AppMode::UtilityMenu => "[1-5] Select Tool | [Esc] Close",
        _ => "[j/k] Up/Down | [h/l] Back/Open | [u] Menu | [f] Find | [s] Settings | [q/Esc] Exit",
    };

    // Formatted to fit cleanly in a standard terminal width
    let content = format!(" Log:   {}\n Guide: {}", status_msg, guide);

    let footer = Paragraph::new(content).block(
        Block::default()
            .title(" System Status & Navigation ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green)),
    );
    frame.render_widget(footer, area);
}

fn render_utility_menu(frame: &mut Frame, _app: &App) {
    let popup_area = centered_rect(60, 15, frame.size());
    frame.render_widget(Clear, popup_area);

    let tabs = vec![
        "[1] Sort",
        "[2] Rename",
        "[3] Clean",
        "[4] Duplicates",
        "[5] Large Files",
    ];
    let mut text = String::new();
    text.push_str("\n Select routine context sequence to run across directory space:\n\n");

    for tab in tabs {
        text.push_str(&format!("    {}\n", tab));
    }
    text.push_str("\n\n Press [Esc/q] to safely yield operation loop context.");

    let paragraph = Paragraph::new(text).block(
        Block::default()
            .title(" Central Utility Toolkit Automation Orchestrator Pipeline ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::LightMagenta)),
    );
    frame.render_widget(paragraph, popup_area);
}

fn render_dry_run_modal(frame: &mut Frame, app: &App) {
    let popup = centered_rect(110, 24, frame.size());
    frame.render_widget(Clear, popup);

    let mut content = String::new();
    content.push_str(" Verified Pipeline Processing Changes Staging Log Manifest:\n\n");

    for (i, change) in app
        .pending_changes
        .iter()
        .skip(app.show_dry_run_scroll)
        .enumerate()
    {
        if i > 16 {
            content.push_str(" ... truncated logs overflow parameters ...\n");
            break;
        }
        content.push_str(&format!("  ● {}\n", change.description));
    }
    content.push_str(
        "\n Execute and flush pipeline operations directly to disk partition safely? (y/n)",
    );

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(" Staged Storage Changes Manifest Log Preview ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red)),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, popup);
}

fn render_settings(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .settings_categories
        .iter()
        .enumerate()
        .map(|(i, cat)| {
            let path = app
                .config
                .sort_targets
                .get(cat)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "Target route missing".to_string());

            // Dynamically gather extensions for display
            let mut exts: Vec<String> = app
                .config
                .extension_map
                .iter()
                .filter(|(_, c)| *c == cat)
                .map(|(ext, _)| ext.clone())
                .collect();
            exts.sort();
            let ext_str = if exts.is_empty() {
                "None".to_string()
            } else {
                exts.join(", ")
            };

            let style = if i == app.settings_selected {
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            // Two-line layout per category for readability
            let text = format!(" [{}] -> {}\n      Extensions: {}", cat, path, ext_str);
            ListItem::new(text).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Sort Module Environment Configuration ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green)),
    );

    frame.render_widget(list, area);
}
fn render_input_modal(frame: &mut Frame, app: &App) {
    let area = centered_rect(55, 3, frame.size());
    frame.render_widget(Clear, area);

    // Converted to Strings so we can dynamically format the Rename state
    let title = match app.input_purpose {
        InputPurpose::Filter => " Filter Files (Regex) ".to_string(),
        InputPurpose::RenamePrefix => format!(
            " Enter New Prefix (Tab to cycle format: {:?}) ",
            app.rename_case
        ),
        InputPurpose::SettingsPath => " Enter Target Directory Path ".to_string(),
        InputPurpose::EditExtensions => " Edit Extensions (comma separated) ".to_string(),
        InputPurpose::NewCategory => " Enter New Category Name ".to_string(),
    };

    let input_text = format!(" > {}_", app.input_buffer);

    let paragraph = Paragraph::new(input_text).block(
        Block::default()
            .title(title) // ratatui natively accepts Strings for titles
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(paragraph, area);
}
