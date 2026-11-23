use std::path::PathBuf;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export, repr(enum = name)))]
pub enum FolderType {
    Recent,
    Desktop,
    Downloads,
    Documents,
    Music,
    Pictures,
    Videos,
}

static ALL_FOLDERS: [FolderType; 7] = [
    FolderType::Recent,
    FolderType::Desktop,
    FolderType::Downloads,
    FolderType::Documents,
    FolderType::Music,
    FolderType::Pictures,
    FolderType::Videos,
];

impl FolderType {
    pub fn values() -> &'static [FolderType] {
        &ALL_FOLDERS
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export))]
pub struct FolderChangedArgs {
    pub of_folder: FolderType,
    pub content: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export))]
pub struct DesktopEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_directory: bool,
    /// True only for executables or shortcuts resolving to applications, not folder/document links.
    pub is_application: bool,
    pub extension: Option<String>,
    /// Localized Windows Shell type name, used by the desktop's type sort.
    pub type_name: Option<String>,
    /// File size in bytes; directories are not recursively measured.
    pub size: Option<u64>,
    /// Last modification time as Unix epoch milliseconds.
    pub modified_at: Option<i64>,
    /// True for entries enumerated from the merged Windows desktop.
    pub is_desktop_entry: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export))]
pub struct DesktopNewFileType {
    pub extension: String,
    pub label: String,
}

/// Result of the actual Explorer desktop background menu. View fields contain
/// changes or the setting explicitly chosen in View/Sort. An unrelated native
/// command never overwrites the organizer's presentation with Explorer defaults.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export))]
pub struct DesktopBackgroundMenuResult {
    pub action: Option<String>,
    pub shell_command: bool,
    pub shell_verb: Option<String>,
    pub icon_size: Option<i32>,
    pub icons_visible: Option<bool>,
    pub auto_arrange: Option<bool>,
    pub snap_to_grid: Option<bool>,
    pub sort: Option<String>,
    pub sort_descending: Option<bool>,
}

/// Organizer preferences applied only to an owned, non-persisting native
/// desktop view. These must never reset the live Explorer desktop's state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export))]
pub struct DesktopBackgroundViewState {
    pub icon_size: i32,
    pub auto_arrange: Option<bool>,
    pub snap_to_grid: Option<bool>,
    pub sort: Option<String>,
    pub sort_descending: Option<bool>,
}

/// Observation of an unconsumed native desktop pointer gesture. Coordinates
/// and viewport are in the owned desktop HWND's physical client pixels.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export))]
pub struct DesktopNativePointer {
    pub phase: String,
    pub gesture_id: u32,
    pub x: f64,
    pub y: f64,
    pub viewport: [f64; 2],
    pub ctrl_key: bool,
    pub shift_key: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export))]
pub struct User {
    pub name: String,
    pub domain: String,
    pub profile_home_path: PathBuf,
    pub email: Option<String>,
    pub one_drive_path: Option<PathBuf>,
    pub profile_picture_path: Option<PathBuf>,
    pub xbox_gamertag: Option<String>,
}
