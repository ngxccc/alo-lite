//! Conversation (`Thread`) and message (`ChatMessage`) domain entities, identity deduplication,
//! sidebar presentation rules, and thread-level pin/hide/delete mutations.

pub mod contact;
pub mod entity;
pub mod merge;
pub mod message;
pub mod mutations;

pub use contact::{
    avatar_file_key, avatar_http_url_count, avatar_url_time, is_generic_thread_name,
    should_replace_avatar, threads_represent_same_contact,
};
pub use entity::{sidebar_sender_id, SidebarFontStyle, SidebarMember, SidebarPresentation, Thread};
pub(super) use merge::MessageIdentityIndex;
pub use merge::{deduplicate_threads, merge_sent_message, merge_thread_into};
pub use message::ChatMessage;
pub use mutations::{
    delete_conversation, get_pinned, set_hidden_conversation, update_pinned_conversation,
};
