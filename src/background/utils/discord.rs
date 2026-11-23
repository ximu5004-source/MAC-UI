use crate::error::Result;

// Compatibility entry points for existing settings; no external presence service.
pub fn start_discord_rpc() -> Result<()> {
    Ok(())
}
pub fn update_discord_rpc(_enabled: bool) -> Result<()> {
    Ok(())
}
