//! Zalo protocol implementation: crypto, session, auth, messaging, and websocket transport.

pub mod auth;
pub mod client;
pub mod crypto;
pub mod endpoints;
pub mod messaging;
pub mod param_cipher;
pub mod registry;
pub mod session;
pub mod social;
pub mod socket;
pub mod sync;
pub mod wire;

pub use auth::{QrConfirmResult, QrLogin, QrScannedProfile};
pub use client::ZaloClient;
pub use messaging::{
    ChatMessage, CloudMessagesPage, DeliveryState, LastMsgsBatch, LinkMetadata, MessageKind,
    MuteSnapshot, PollBatch, PollState, StickerItem, StickerPackInfo, Thread, UnreadPolicy,
    WireMessageRef,
};
pub use registry::{ApiDomain, CommandSpec};
pub use session::Session;
pub use social::{FriendshipStatus, PhoneLookupResult};
pub use wire::{ActionAck, SendMessageReceipt};
