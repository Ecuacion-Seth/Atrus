use crate::config::AppConfig;
use crate::engine;
use crate::models::*;
use std::path::PathBuf;

pub struct App {
    pub current_dir: PathBuf,
    pub files: Vec<FileItem>,
    pub filtered_files: Vec<FileItem>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub mode: AppMode,
    pub undo_stack: Vec<Vec<UndoLog>>,
    pub config: AppConfig,
    pub search_query: String,
    pub pending_changes: Vec<PendingChange>,
    pub utility_tab: UtilityTab,
    pub rename_prefix: String,
    pub rename_case: RenameCase,
    pub message: String,
    pub right_panel_view: RightPanelView,
    pub large_files: Vec<(PathBuf, u64)>,
    pub duplicates: Vec<Vec<PathBuf>>,
    pub input_purpose: InputPurpose,
    pub input_buffer: String,
    pub settings_categories: Vec<String>,
    pub settings_selected: usize,
    pub show_dry_run_scroll: usize,
    pub last_key_time: std::time::Instant,
    pub regex_valid: bool, // NEW: Safely tracks the fallback state
}

impl App {
    pub fn new() -> anyhow::Result<Self> {
        let current_dir = std::env::current_dir()?;
        let config = AppConfig::load()?;
        let mut app = Self {
            current_dir,
            files: Vec::new(),
            filtered_files: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            mode: AppMode::Explorer,
            undo_stack: Vec::new(),
            config,
            search_query: String::new(),
            pending_changes: Vec::new(),
            utility_tab: UtilityTab::Sort,
            rename_prefix: String::new(),
            rename_case: RenameCase::Lowercase,
            message: "Welcome to TUI File Manager!".to_string(),
            right_panel_view: RightPanelView::FileMetadata,
            large_files: Vec::new(),
            duplicates: Vec::new(),
            input_purpose: InputPurpose::Filter,
            input_buffer: String::new(),
            settings_categories: Vec::new(),
            settings_selected: 0,
            show_dry_run_scroll: 0,
            last_key_time: std::time::Instant::now(),
            regex_valid: true, // Default to true
        };
        app.refresh_files()?;
        Ok(app)
    }
    pub fn refresh_files(&mut self) -> anyhow::Result<()> {
        self.files = engine::read_directory(&self.current_dir)?;
        self.apply_filter()?;
        // Safely bounds-check selection and offset after folder change
        if self.selected_index >= self.filtered_files.len() {
            self.selected_index = 0;
            self.scroll_offset = 0;
        }
        Ok(())
    }

    pub fn apply_filter(&mut self) -> anyhow::Result<()> {
        if self.search_query.is_empty() {
            self.filtered_files = self.files.clone();
            self.regex_valid = true;
        } else {
            let (filtered, used_fallback) = engine::filter_files(&self.files, &self.search_query)?;
            self.filtered_files = filtered;
            self.regex_valid = !used_fallback;

            if !self.regex_valid {
                self.message = "(Plain text match — invalid regex)".to_string();
            }
        }

        if self.selected_index >= self.filtered_files.len() {
            self.selected_index = self.filtered_files.len().saturating_sub(1);
        }
        Ok(())
    }

    /// Dynamically shifts the scroll viewport based on terminal render bounds
    pub fn adjust_viewport(&mut self, visible_height: usize) {
        if self.filtered_files.is_empty() {
            self.scroll_offset = 0;
            return;
        }

        // Clip selection to absolute bounds
        if self.selected_index >= self.filtered_files.len() {
            self.selected_index = self.filtered_files.len().saturating_sub(1);
        }

        // If selection went above the window view boundary
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        }
        // If selection went below the window view boundary
        else if self.selected_index >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected_index - visible_height + 1;
        }
    }

    pub fn move_selection_down(&mut self) {
        if !self.filtered_files.is_empty() {
            if self.selected_index + 1 < self.filtered_files.len() {
                self.selected_index += 1;
            } else {
                self.selected_index = 0; // Wrap around to top
                self.scroll_offset = 0;
            }
        }
    }

    pub fn move_selection_up(&mut self) {
        if !self.filtered_files.is_empty() {
            if self.selected_index > 0 {
                self.selected_index -= 1;
            } else {
                self.selected_index = self.filtered_files.len().saturating_sub(1);
                // Wrap around to bottom
            }
        }
    }

    pub fn go_into_dir(&mut self) -> anyhow::Result<()> {
        if let Some(file) = self.filtered_files.get(self.selected_index) {
            if file.is_dir {
                self.current_dir = file.path.clone();
                self.selected_index = 0;
                self.scroll_offset = 0;
                self.refresh_files()?;
            }
        }
        Ok(())
    }

    pub fn go_to_parent(&mut self) -> anyhow::Result<()> {
        if let Some(parent) = self.current_dir.parent() {
            let old_dir = self.current_dir.clone();
            self.current_dir = parent.to_path_buf();
            self.refresh_files()?;

            // Re-highlight the folder we just exited
            if let Some(pos) = self.filtered_files.iter().position(|f| f.path == old_dir) {
                self.selected_index = pos;
                self.scroll_offset = pos.saturating_sub(5); // Bring safely into view
            } else {
                self.selected_index = 0;
                self.scroll_offset = 0;
            }
        }
        Ok(())
    }

    pub fn run_utility_logic(&mut self) -> anyhow::Result<()> {
        match self.utility_tab {
            UtilityTab::Sort => {
                self.pending_changes =
                    engine::generate_sort_changes(&self.current_dir, &self.config)?;
                if self.pending_changes.is_empty() {
                    self.message = "All items are already sorted cleanly.".to_string();
                    self.mode = AppMode::Explorer;
                } else {
                    self.mode = AppMode::DryRunPreview;
                    self.show_dry_run_scroll = 0;
                    self.message = format!(
                        "Staged {} organization operations. Confirm? (y/n)",
                        self.pending_changes.len()
                    );
                }
            }
            UtilityTab::Clean => {
                self.pending_changes =
                    engine::generate_cleanup_changes(&self.current_dir, &self.config)?;
                if self.pending_changes.is_empty() {
                    self.message = "No empty folders or broken links detected.".to_string();
                    self.mode = AppMode::Explorer;
                } else {
                    self.mode = AppMode::DryRunPreview;
                    self.show_dry_run_scroll = 0;
                    self.message = format!(
                        "Staged {} deletions. Confirm structural removal? (y/n)",
                        self.pending_changes.len()
                    );
                }
            }
            UtilityTab::Rename => {
                self.input_purpose = InputPurpose::RenamePrefix;
                self.input_buffer = String::new();
                self.mode = AppMode::FilterInput;
            }
            UtilityTab::Duplicates => {
                self.duplicates = engine::find_duplicate_files(&self.current_dir)?;
                self.right_panel_view = RightPanelView::Duplicates;
                self.message = format!(
                    "Scanning finished: {} duplicate signature groups found.",
                    self.duplicates.len()
                );
                self.mode = AppMode::Explorer;
            }
            UtilityTab::LargeFiles => {
                self.large_files = engine::find_large_files(&self.current_dir, 20)?;
                self.right_panel_view = RightPanelView::LargeFiles;
                let total_size: u64 = self.large_files.iter().map(|(_, s)| s).sum();
                self.message = format!("Top 20 large files found. Total: {} bytes.", total_size);
                self.mode = AppMode::Explorer;
            }
        }
        Ok(())
    }

    pub fn execute_rename(&mut self) -> anyhow::Result<()> {
        self.pending_changes = engine::batch_rename_dry_run(
            &self.current_dir,
            &self.rename_prefix,
            &self.rename_case,
        )?;
        if self.pending_changes.is_empty() {
            self.message = "No files to rename.".to_string();
            self.mode = AppMode::Explorer;
        } else {
            self.mode = AppMode::DryRunPreview;
            self.show_dry_run_scroll = 0;
            self.message = format!(
                "{} files to rename. Confirm? (y/n)",
                self.pending_changes.len()
            );
        }
        Ok(())
    }

    pub fn confirm_pending_changes(&mut self) -> anyhow::Result<()> {
        let (logs, errors) = engine::apply_changes(&self.pending_changes)?;

        if !logs.is_empty() {
            self.undo_stack.push(logs);
        }

        // Clear staging and reset mode
        self.pending_changes.clear();
        self.mode = AppMode::Explorer;

        // Refresh disk state FIRST
        self.refresh_files()?;

        // Assert the actual operation result LAST so it wins the render cycle
        if errors.is_empty() {
            self.message = "Successfully processed all changes.".to_string();
        } else {
            self.message = format!(
                "Processed changes, but {} error(s) (e.g., {}).",
                errors.len(),
                errors.first().unwrap_or(&"unknown".to_string())
            );
        }

        Ok(())
    }

    pub fn cancel_pending_changes(&mut self) {
        self.pending_changes.clear();
        self.mode = AppMode::Explorer;
        self.message = "Operation cancelled safely.".to_string();
    }

    pub fn undo_last(&mut self) -> anyhow::Result<()> {
        if let Some(logs) = self.undo_stack.pop() {
            engine::undo_operations(&logs)?;

            // Refresh disk state FIRST
            self.refresh_files()?;

            // Set message LAST so it overwrites the "Idle" default
            self.message = "Last operation successfully reversed.".to_string();
        } else {
            self.message = "Undo structural stack is empty!".to_string();
        }
        Ok(())
    }

    pub fn load_settings(&mut self) {
        self.settings_categories = self.config.sort_targets.keys().cloned().collect();
        self.settings_categories.sort();
        self.settings_selected = 0;
    }

    pub fn save_settings(&mut self) -> anyhow::Result<()> {
        self.config.save()?;
        self.message = "Configuration successfully flushed to disk storage JSON.".to_string();
        self.mode = AppMode::Explorer;
        Ok(())
    }
}
