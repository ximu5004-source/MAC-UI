use chrono::NaiveTime;

/// Night Light (blue light reduction) schedule mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export))]
pub enum ScheduleMode {
    Off,
    SunsetToSunrise,
    SetHours,
}

/// Night Light settings, stored in the registry as a Bond CompactBinary v1 payload.
///
/// Mirrors `NightlightSettings` from <https://github.com/kvnxiao/win-nightlight-cli>, which
/// reverse-engineered this format; the parsing/serialization logic lives in
/// `modules::system_settings` on the background crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export))]
pub struct NightlightSettings {
    /// The last-modified Unix timestamp in seconds
    pub timestamp: u64,
    /// The schedule mode
    pub schedule_mode: ScheduleMode,
    /// The color temperature in Kelvin
    pub color_temperature: u16,
    /// The start time of the schedule when [schedule_mode](Self::schedule_mode) is [ScheduleMode::SetHours]
    pub start_time: NaiveTime,
    /// The end time of the schedule when [schedule_mode](Self::schedule_mode) is [ScheduleMode::SetHours]
    pub end_time: NaiveTime,
    /// The sunset time
    pub sunset_time: NaiveTime,
    /// The sunrise time
    pub sunrise_time: NaiveTime,
}
