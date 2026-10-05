//! Windows Night Light (blue light reduction) binary registry codec.
//!
//! Windows persists Night Light settings/state in `HKCU` as a Bond CompactBinary v1
//! payload wrapped in a small CloudStore envelope. This module only implements that
//! generic binary codec (`bond` + `cloudstore`); the Night Light–specific field layout,
//! parsing and registry I/O live in [`super::application`].
//!
//! Ported and adapted from <https://github.com/kvnxiao/win-nightlight-cli>
//! (`win-nightlight-lib`, MIT licensed), which reverse-engineered this format.

pub mod bond;
pub mod cloudstore;
mod manager;

pub use manager::*;
