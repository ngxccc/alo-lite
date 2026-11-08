//! QR login — `GET /api/login/reqqr` → render → wait for scan → confirm.
//!
//! Evidence: `re-notes/01_login.md` §5.2, plus the live capture in the tests below.
//! The owner of this analysis is the maker of the client on this machine (Zalo PC
//! 26.10.11, extracted to `asar_extracted/`).

use super::{login, trust};

pub mod builder;
pub mod flow;
pub mod polling;
pub mod render;
pub mod types;

#[cfg(test)]
mod tests;

pub use builder::{
    req_qr_params, req_qr_params_form, request_qr, request_qr_plain, request_qr_trusted,
};
pub use flow::login_via_qr;
pub use polling::{confirm, wait_for_scan};
pub use render::{render_terminal, save_png};
pub use types::{extract_zpw_sek, CompanionState, QrConfirmResult, QrLogin, QrScannedProfile};

#[cfg(test)]
pub(crate) use builder::reqqr_url;
#[cfg(test)]
pub(crate) use polling::process_companion_link_data;
