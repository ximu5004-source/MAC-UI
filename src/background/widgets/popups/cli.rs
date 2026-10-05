pub use slu_ipc::commands::PopupsCli;
use slu_ipc::commands::PopupsCommand;

use crate::{error::Result, widgets::popups::shortcut_registering::set_registering_shortcut};

pub fn process(cmd: PopupsCli) -> Result<()> {
    #[allow(clippy::single_match)]
    match cmd.subcommand {
        PopupsCommand::InternalSetShortcut { json, request_id } => {
            // An old service cannot identify which registration it is replying
            // to. Never let its delayed keys/cancellation affect a new capture.
            let Some(request_id) = request_id else {
                return Ok(());
            };
            let shortcut: Option<Vec<String>> = serde_json::from_str(&json)?;
            set_registering_shortcut(request_id, shortcut)?;
        }
        _ => {}
    }
    Ok(())
}
