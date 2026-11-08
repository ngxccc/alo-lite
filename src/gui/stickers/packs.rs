// ---------------------------------------------------------------------------
// Sticker Presets & State Types (shared definitions live in `crate::core::stickers`)
// ---------------------------------------------------------------------------

pub use crate::core::stickers::{
    builtin_pack_stickers, export_custom_stickers_template, load_custom_sticker_packs,
    CustomStickerPack, AMI_FAT_CAT_CATE_ID, FEATURED_PACK_TABS, ONION_STICKERS, POPULAR_STICKERS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StickerTab {
    Popular,
    Onion,
    Pack(u64),
    Search,
    Store,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadFilter {
    All,
    Direct,
    Groups,
}
