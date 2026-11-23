pub use macui_ipc::commands::WallpaperCli;
use macui_ipc::commands::WallpaperCommand;

use crate::{error::Result, virtual_desktops::wallpapers::WorkspaceWallpapersManager};

pub fn process(cmd: WallpaperCli) -> Result<()> {
    match cmd.command {
        WallpaperCommand::Next => WorkspaceWallpapersManager::next(),
        WallpaperCommand::Prev => WorkspaceWallpapersManager::previous(),
    }
    Ok(())
}
