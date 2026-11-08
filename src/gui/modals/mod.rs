pub mod calls;
pub mod dialogs;
pub mod feed;
pub mod phone_lookup;
pub mod profile;
pub mod scheduler;
pub mod settings;
pub mod updates;

#[allow(unused_imports)]
pub(crate) use calls::{
    compute_call_fit_aspects, compute_call_gallery_tiles, compute_call_speaker_layout,
};
