pub use macui_ipc::commands::WidgetCli;
use macui_ipc::commands::WidgetCommand;

use macui_core::state::WidgetTriggerPayload;

use crate::{error::Result, widgets::trigger_widget};

pub fn run(cmd: WidgetCli) -> Result<()> {
    match cmd.command {
        WidgetCommand::Trigger { widget_id } => {
            trigger_widget(WidgetTriggerPayload::new(widget_id.into()))?;
        }
    }
    Ok(())
}
