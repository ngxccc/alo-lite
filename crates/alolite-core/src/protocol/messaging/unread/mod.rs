//! Unread state follows server read cursors, never the size of a history batch.

pub mod marks;
pub mod mentions;
pub mod mute;
pub mod state;

pub use marks::{
    fetch_manual_unread_marks, merge_manual_unread_marks, merge_read_points,
    parse_manual_unread_marks, send_seen_v2, set_unread_mark, ManualUnreadMark,
};
pub use mentions::{
    classify_message_mention, detect_message_mention, MentionDetection, MENTION_ALL_ID,
};
pub use mute::{
    extract_mute_controls, extract_mute_map, get_mute_conversations, get_mute_snapshot,
    merge_mute_metadata, set_mute_conversation, MuteControlEvent, MuteSnapshot,
};
pub use state::{compare_ids, UnreadState};
