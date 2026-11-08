//! HTTP long-polling (`getChatFromPolling`), `/api/preloadconvers/get-last-msgs`,
//! cloud history (`/api/cm/getrecentv2` / `getoldv2`), and wire message parser.

pub mod batch;
pub mod fetch;
pub mod merge;
pub mod parse;
pub mod state;

pub use batch::{CloudMessagesPage, LastMsgsBatch, PollBatch};
pub use fetch::{get_chat_from_polling, get_cloud_messages, get_last_msgs, poll_new_messages};
pub use merge::{merge_group_history, merge_previews};
pub use state::{PollState, QueueCursor};
