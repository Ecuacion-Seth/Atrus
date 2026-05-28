use crate::app::App;
use crate::models::{AppMode, InputPurpose, RenameCase, UtilityTab};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};

pub fn handle_events(app: &mut App) -> anyhow::Result<bool> {
    // Lowered polling delay to 16ms (~60Hz) for instant responsiveness
    if event::poll(std::time::Duration::from_millis(16))? {
        if let Event::Key(key) = event::read()? {
            return Ok(handle_key_event(app, key));
        }
    }
    Ok(false)
}

fn handle_key_event(app: &mut App, key: KeyEvent) -> bool {
    // 1. Filter out key releases - only react when the key is actively pressed down
    if key.kind != KeyEventKind::Press {
        return false;
    }

    // 2. Hardware debouncer: Reduced from 150ms to 16ms
    if app.last_key_time.elapsed() < std::time::Duration::from_millis(16) {
        return false;
    }
    app.last_key_time = std::time::Instant::now();

    match app.mode {
        AppMode::Explorer => handle_explorer_keys(app, key),
        AppMode::UtilityMenu => handle_utility_menu_keys(app, key),
        AppMode::FilterInput => handle_filter_input_keys(app, key),
        AppMode::DryRunPreview => handle_dry_run_keys(app, key),
        AppMode::Settings => handle_settings_keys(app, key),
    }
}

fn handle_explorer_keys(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return true,
        KeyCode::Char('j') | KeyCode::Down => app.move_selection_down(),
        KeyCode::Char('k') | KeyCode::Up => app.move_selection_up(),
        KeyCode::Char('h') | KeyCode::Left | KeyCode::Backspace => {
            let _ = app.go_to_parent();
        }
        KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => {
            let _ = app.go_into_dir();
        }
        KeyCode::Char('f') => {
            app.input_purpose = InputPurpose::Filter;
            app.input_buffer = app.search_query.clone();
            app.mode = AppMode::FilterInput;
        }
        KeyCode::Char('u') => {
            app.mode = AppMode::UtilityMenu;
        }
        KeyCode::Char('s') => {
            app.load_settings();
            app.mode = AppMode::Settings;
        }
        // Structural safe runtime inverse mapping operations undo trigger execution
        KeyCode::Char('z') => {
            let _ = app.undo_last();
        }
        _ => {}
    }
    false
}

fn handle_utility_menu_keys(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = AppMode::Explorer;
        }
        KeyCode::Char('1') => {
            app.utility_tab = UtilityTab::Sort;
            let _ = app.run_utility_logic();
        }
        KeyCode::Char('2') => {
            app.utility_tab = UtilityTab::Rename;
            let _ = app.run_utility_logic();
        }
        KeyCode::Char('3') => {
            app.utility_tab = UtilityTab::Clean;
            let _ = app.run_utility_logic();
        }
        KeyCode::Char('4') => {
            app.utility_tab = UtilityTab::Duplicates;
            let _ = app.run_utility_logic();
        }
        KeyCode::Char('5') => {
            app.utility_tab = UtilityTab::LargeFiles;
            let _ = app.run_utility_logic();
        }
        _ => {}
    }
    false
}

fn handle_filter_input_keys(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.mode = AppMode::Explorer;
        }
        KeyCode::Tab => {
            if app.input_purpose == InputPurpose::RenamePrefix {
                app.rename_case = match app.rename_case {
                    RenameCase::Lowercase => RenameCase::Uppercase,
                    RenameCase::Uppercase => RenameCase::CamelCase,
                    RenameCase::CamelCase => RenameCase::Lowercase,
                };
            }
        }
        KeyCode::Enter => match app.input_purpose {
            InputPurpose::Filter => {
                app.search_query = app.input_buffer.clone();
                let _ = app.refresh_files();

                // Safely evaluate typed state
                if app.regex_valid && !app.search_query.is_empty() {
                    app.message = format!("Applied regex filter: '{}'", app.search_query);
                }

                app.mode = AppMode::Explorer;
            }
            InputPurpose::RenamePrefix => {
                app.rename_prefix = app.input_buffer.clone();
                let _ = app.execute_rename();
            }
            InputPurpose::SettingsPath => {
                if let Some(category) = app.settings_categories.get(app.settings_selected) {
                    app.config.sort_targets.insert(
                        category.clone(),
                        std::path::PathBuf::from(&app.input_buffer),
                    );
                }
                app.mode = AppMode::Settings;
            }
            InputPurpose::EditExtensions => {
                if let Some(category) = app.settings_categories.get(app.settings_selected).cloned()
                {
                    app.config.extension_map.retain(|_, cat| cat != &category);
                    for ext in app.input_buffer.split(',') {
                        let cleaned_ext = ext.trim().to_lowercase().replace(".", "");
                        if !cleaned_ext.is_empty() {
                            app.config
                                .extension_map
                                .insert(cleaned_ext, category.clone());
                        }
                    }
                }
                app.mode = AppMode::Settings;
            }
            InputPurpose::NewCategory => {
                let cat_name = app.input_buffer.trim().to_string();
                if !cat_name.is_empty() && !app.settings_categories.contains(&cat_name) {
                    app.settings_categories.push(cat_name.clone());
                    app.settings_categories.sort();
                    app.config
                        .sort_targets
                        .insert(cat_name.clone(), std::path::PathBuf::from(&cat_name));

                    if let Some(idx) = app.settings_categories.iter().position(|c| c == &cat_name) {
                        app.settings_selected = idx;
                    }
                }
                app.mode = AppMode::Settings;
            }
        },
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        _ => {}
    }
    false
}

fn handle_dry_run_keys(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            let _ = app.confirm_pending_changes();
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.cancel_pending_changes();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.show_dry_run_scroll + 1 < app.pending_changes.len() {
                app.show_dry_run_scroll += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.show_dry_run_scroll > 0 {
                app.show_dry_run_scroll -= 1;
            }
        }
        _ => {}
    }
    false
}

fn handle_settings_keys(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = AppMode::Explorer;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.settings_selected + 1 < app.settings_categories.len() {
                app.settings_selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.settings_selected > 0 {
                app.settings_selected -= 1;
            }
        }
        KeyCode::Char('e') | KeyCode::Enter => {
            if let Some(category) = app.settings_categories.get(app.settings_selected) {
                app.input_purpose = InputPurpose::SettingsPath;
                let current_path = app
                    .config
                    .sort_targets
                    .get(category)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                app.input_buffer = current_path;
                app.mode = AppMode::FilterInput;
            }
        }
        KeyCode::Char('x') => {
            if let Some(category) = app.settings_categories.get(app.settings_selected) {
                app.input_purpose = InputPurpose::EditExtensions;
                // Gather all flat extensions mapped to this category
                let mut exts: Vec<String> = app
                    .config
                    .extension_map
                    .iter()
                    .filter(|(_, cat)| *cat == category)
                    .map(|(ext, _)| ext.clone())
                    .collect();
                exts.sort();
                app.input_buffer = exts.join(", ");
                app.mode = AppMode::FilterInput;
            }
        }
        KeyCode::Char('n') => {
            app.input_purpose = InputPurpose::NewCategory;
            app.input_buffer = String::new();
            app.mode = AppMode::FilterInput;
        }
        KeyCode::Char('d') | KeyCode::Delete => {
            if !app.settings_categories.is_empty() {
                let category = app.settings_categories.remove(app.settings_selected);
                app.config.sort_targets.remove(&category);
                app.config.extension_map.retain(|_, cat| cat != &category);

                // Adjust selection cursor safely
                if app.settings_selected >= app.settings_categories.len()
                    && app.settings_selected > 0
                {
                    app.settings_selected -= 1;
                }
            }
        }
        KeyCode::Char('s') => {
            let _ = app.save_settings();
        }
        _ => {}
    }
    false
}
