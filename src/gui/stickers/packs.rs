// ---------------------------------------------------------------------------
// Sticker Presets & State Types
// ---------------------------------------------------------------------------

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

#[derive(Debug, Clone, Copy)]
pub struct PresetSticker {
    pub sticker_id: u64,
    pub cate_id: u64,
    pub sticker_type: u32,
    pub label: &'static str,
}

/// Built-in featured Zalo sticker packs (`(cate_id, title)`).
pub const AMI_FAT_CAT_CATE_ID: u64 = 99901;

pub const FEATURED_PACK_TABS: &[(u64, &str)] = &[
    (61900, "Zapy Dô Tri"),
    (12815, "Bé Hai"),
    (12748, "Cò Lõ Đi Làm"),
    (12705, "Mèo Meon"),
    (12718, "Gấu Cay"),
    (12045, "Zameo"),
    (11150, "Bé Dúi"),
    (10186, "Đậu Đỏ"),
    (10177, "Heo Luci"),
    (10117, "Chó Toto"),
];

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CustomStickerPack {
    pub cate_id: u64,
    pub name: String,
    pub sticker_ids: Vec<u64>,
    #[serde(default = "default_sticker_type_7")]
    pub sticker_type: u32,
}

fn default_sticker_type_7() -> u32 {
    7
}

pub fn load_custom_sticker_packs() -> Vec<CustomStickerPack> {
    std::fs::read_to_string("custom_stickers.json")
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<CustomStickerPack>>(&s).ok())
        .unwrap_or_default()
}

pub fn export_custom_stickers_template(path: &str) -> std::io::Result<()> {
    let sample = vec![CustomStickerPack {
        cate_id: 99001,
        name: "Custom Pack Mẫu".into(),
        sticker_ids: vec![98930, 98931, 98932, 101001, 101002],
        sticker_type: 7,
    }];
    let text = serde_json::to_string_pretty(&sample).unwrap_or_default();
    std::fs::write(path, text)
}

pub fn builtin_pack_stickers(cate_id: u64) -> Option<Vec<crate::protocol::messaging::StickerItem>> {
    if cate_id == AMI_FAT_CAT_CATE_ID {
        return None;
    }
    for custom in load_custom_sticker_packs() {
        if custom.cate_id == cate_id {
            return Some(
                custom
                    .sticker_ids
                    .into_iter()
                    .enumerate()
                    .map(|(i, sid)| crate::protocol::messaging::StickerItem {
                        sticker_id: sid,
                        cate_id: custom.cate_id,
                        sticker_type: custom.sticker_type,
                        label: format!("{} #{}", custom.name, i + 1),
                    })
                    .collect(),
            );
        }
    }
    let (name, ids): (&str, Vec<u64>) = match cate_id {
        61900 => ("Zapy Dô Tri", (98930..=98941).collect()),
        12815 => ("Bé Hai", (101001..=101030).collect()),
        12748 => ("Cò Lõ Đi Làm", (99978..=99997).collect()),
        12705 => ("Mèo Meon", (99278..=99293).collect()),
        12718 => ("Gấu Cay", (99486..=99501).collect()),
        12045 => (
            "Zameo Rốn Lồi",
            (47614..=47624).chain(47626..=47633).collect(),
        ),
        11150 => ("Bé Dúi", (34107..=34110).chain(34827..=34837).collect()),
        10186 => ("Đậu Đỏ", (18974..=18995).collect()),
        10177 => ("Heo Luci", (18847..=18871).collect()),
        10117 => ("Chó Toto", (17745..=17764).collect()),
        _ => return None,
    };
    Some(
        ids.into_iter()
            .enumerate()
            .map(|(i, sid)| crate::protocol::messaging::StickerItem {
                sticker_id: sid,
                cate_id,
                sticker_type: 7,
                label: format!("{name} #{}", i + 1),
            })
            .collect(),
    )
}

pub const POPULAR_STICKERS: &[PresetSticker] = &[
    PresetSticker {
        sticker_id: 18009,
        cate_id: 10130,
        sticker_type: 7,
        label: "Xin chào",
    },
    PresetSticker {
        sticker_id: 21458,
        cate_id: 10302,
        sticker_type: 7,
        label: "Vẫy tay",
    },
    PresetSticker {
        sticker_id: 21459,
        cate_id: 10302,
        sticker_type: 7,
        label: "Vui vẻ",
    },
    PresetSticker {
        sticker_id: 21460,
        cate_id: 10302,
        sticker_type: 7,
        label: "Thả tim",
    },
    PresetSticker {
        sticker_id: 21461,
        cate_id: 10302,
        sticker_type: 7,
        label: "Cười lớn",
    },
    PresetSticker {
        sticker_id: 21462,
        cate_id: 10302,
        sticker_type: 7,
        label: "Hóng",
    },
    PresetSticker {
        sticker_id: 21463,
        cate_id: 10302,
        sticker_type: 7,
        label: "Ngạc nhiên",
    },
    PresetSticker {
        sticker_id: 21464,
        cate_id: 10302,
        sticker_type: 7,
        label: "Cảm ơn",
    },
    PresetSticker {
        sticker_id: 16713,
        cate_id: 10047,
        sticker_type: 7,
        label: "Hello",
    },
    PresetSticker {
        sticker_id: 18869,
        cate_id: 10177,
        sticker_type: 7,
        label: "OK",
    },
    PresetSticker {
        sticker_id: 19773,
        cate_id: 10233,
        sticker_type: 7,
        label: "Tuyệt",
    },
    PresetSticker {
        sticker_id: 17751,
        cate_id: 10117,
        sticker_type: 7,
        label: "Yêu thương",
    },
    PresetSticker {
        sticker_id: 18910,
        cate_id: 10183,
        sticker_type: 7,
        label: "Haha",
    },
    PresetSticker {
        sticker_id: 20695,
        cate_id: 10275,
        sticker_type: 7,
        label: "Chơi game",
    },
    PresetSticker {
        sticker_id: 19949,
        cate_id: 10242,
        sticker_type: 7,
        label: "Mèo con",
    },
    PresetSticker {
        sticker_id: 394,
        cate_id: 14,
        sticker_type: 7,
        label: "Chào bạn",
    },
    PresetSticker {
        sticker_id: 98930,
        cate_id: 61900,
        sticker_type: 7,
        label: "Zapy Xin chào",
    },
    PresetSticker {
        sticker_id: 101001,
        cate_id: 12815,
        sticker_type: 7,
        label: "Bé Hai #1",
    },
    PresetSticker {
        sticker_id: 99978,
        cate_id: 12748,
        sticker_type: 7,
        label: "Cò Lõ #1",
    },
    PresetSticker {
        sticker_id: 99278,
        cate_id: 12705,
        sticker_type: 7,
        label: "Mèo Meon #1",
    },
    PresetSticker {
        sticker_id: 99486,
        cate_id: 12718,
        sticker_type: 7,
        label: "Gấu Cay #1",
    },
    PresetSticker {
        sticker_id: 47614,
        cate_id: 12045,
        sticker_type: 7,
        label: "Zameo #1",
    },
    PresetSticker {
        sticker_id: 34107,
        cate_id: 11150,
        sticker_type: 7,
        label: "Bé Dúi #1",
    },
    PresetSticker {
        sticker_id: 18974,
        cate_id: 10186,
        sticker_type: 7,
        label: "Đậu Đỏ #1",
    },
];

pub const ONION_STICKERS: &[PresetSticker] = &[
    PresetSticker {
        sticker_id: 1,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #1",
    },
    PresetSticker {
        sticker_id: 2,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #2",
    },
    PresetSticker {
        sticker_id: 3,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #3",
    },
    PresetSticker {
        sticker_id: 4,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #4",
    },
    PresetSticker {
        sticker_id: 5,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #5",
    },
    PresetSticker {
        sticker_id: 6,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #6",
    },
    PresetSticker {
        sticker_id: 7,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #7",
    },
    PresetSticker {
        sticker_id: 8,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #8",
    },
    PresetSticker {
        sticker_id: 9,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #9",
    },
    PresetSticker {
        sticker_id: 10,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #10",
    },
    PresetSticker {
        sticker_id: 11,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #11",
    },
    PresetSticker {
        sticker_id: 12,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #12",
    },
    PresetSticker {
        sticker_id: 13,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #13",
    },
    PresetSticker {
        sticker_id: 14,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #14",
    },
    PresetSticker {
        sticker_id: 15,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #15",
    },
    PresetSticker {
        sticker_id: 16,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #16",
    },
    PresetSticker {
        sticker_id: 17,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #17",
    },
    PresetSticker {
        sticker_id: 18,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #18",
    },
    PresetSticker {
        sticker_id: 19,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #19",
    },
    PresetSticker {
        sticker_id: 20,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #20",
    },
    PresetSticker {
        sticker_id: 21,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #21",
    },
    PresetSticker {
        sticker_id: 22,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #22",
    },
    PresetSticker {
        sticker_id: 23,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #23",
    },
    PresetSticker {
        sticker_id: 24,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #24",
    },
    PresetSticker {
        sticker_id: 25,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #25",
    },
    PresetSticker {
        sticker_id: 26,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #26",
    },
    PresetSticker {
        sticker_id: 27,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #27",
    },
    PresetSticker {
        sticker_id: 28,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #28",
    },
    PresetSticker {
        sticker_id: 29,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #29",
    },
    PresetSticker {
        sticker_id: 30,
        cate_id: 0,
        sticker_type: 7,
        label: "Củ hành #30",
    },
];
