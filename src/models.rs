use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct FileItem {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub is_dir: bool,
    pub permissions: String,
    pub modified: String,
}

#[derive(Debug, Clone)]
pub struct UndoLog {
    pub old_path: PathBuf,
    pub new_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChangeKind {
    Move,
    DeleteDir,
}

#[derive(Debug, Clone)]
pub struct PendingChange {
    pub src: PathBuf,
    pub dst: PathBuf,
    pub description: String,
    pub kind: ChangeKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Explorer,
    UtilityMenu,
    FilterInput,
    DryRunPreview,
    Settings,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UtilityTab {
    Sort,
    Rename,
    Clean,
    Duplicates,
    LargeFiles,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RenameCase {
    Lowercase,
    Uppercase,
    CamelCase,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputPurpose {
    Filter,
    RenamePrefix,
    SettingsPath,
    EditExtensions, // NEW
    NewCategory,    // NEW
}

#[derive(Debug, Clone, PartialEq)]
pub enum RightPanelView {
    FileMetadata,
    LargeFiles,
    Duplicates,
}
