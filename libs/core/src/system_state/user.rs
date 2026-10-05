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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export))]
pub struct DesktopNewFileType {
    pub extension: String,
    pub label: String,
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
