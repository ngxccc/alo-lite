//! Centralized Zalo Client API.
//!
//! Provides a unified, thread-safe interface for all Zalo protocol operations:
//! - Authentication & QR login
//! - Friends & contact profiles
//! - Group management
//! - Cloud and local message history
//! - Direct & group messaging (text, attachments, links, stickers, quotes)
//! - Message actions (reactions, deletions, recalls, forwards)
//! - Mobile synchronization and polling

use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

use anyhow::Result;
use serde_json::Value;

use crate::protocol::auth::{QrConfirmResult, QrLogin, QrScannedProfile};
use crate::protocol::messaging::{
    ChatMessage, CloudMessagesPage, LastMsgsBatch, LinkMetadata, PollState, StickerItem,
    StickerPackInfo, Thread,
};
use crate::protocol::session::Session;
use crate::protocol::social::{FriendshipStatus, PhoneLookupResult};
use crate::protocol::wire::{ActionAck, SendMessageReceipt};
/// High-level client for the Zalo desktop protocol.
///
/// Encapsulates authentication session state in an [`Arc<Mutex<Session>>`]
/// and exposes strongly-typed, consolidated asynchronous API methods.
#[derive(Clone)]
pub struct ZaloClient {
    session: Arc<Mutex<Session>>,
}

impl ZaloClient {
    /// Create a new `ZaloClient` from an existing shared session.
    pub fn new(session: Arc<Mutex<Session>>) -> Self {
        Self { session }
    }

    /// Create a new `ZaloClient` owning an isolated session.
    pub fn from_session(session: Session) -> Self {
        Self {
            session: Arc::new(Mutex::new(session)),
        }
    }

    /// Access the underlying shared session mutex.
    pub fn session(&self) -> &Arc<Mutex<Session>> {
        &self.session
    }

    /// Snapshot the current session state.
    pub async fn snapshot_session(&self) -> Session {
        self.session.lock().await.clone()
    }

    // ------------------------------------------------------------------------
    // Authentication & QR Login
    // ------------------------------------------------------------------------

    /// Request a new QR code ticket from Zalo auth servers.
    pub async fn request_qr(&self, language: &str) -> Result<QrLogin> {
        let s = self.session.lock().await.clone();
        crate::protocol::auth::qr::request_qr(&s, language).await
    }

    /// Long-poll waiting for the user to scan the QR code.
    pub async fn wait_for_qr_scan(&self, q: &mut QrLogin) -> Result<QrScannedProfile> {
        let s = self.session.lock().await.clone();
        crate::protocol::auth::qr::wait_for_scan(&s, q).await
    }

    /// Poll until the user confirms the QR login on mobile.
    pub async fn confirm_qr(&self, q: &QrLogin) -> Result<QrConfirmResult> {
        let s = self.session.lock().await.clone();
        crate::protocol::auth::qr::confirm(&s, q).await
    }

    /// Query login info with session cookies (`/api/login/getLoginInfo`).
    pub async fn get_login_info(&self) -> Result<Value> {
        let s = self.session.lock().await.clone();
        crate::protocol::auth::login::get_login_info(&s).await
    }

    /// Fetch server configuration (`/api/login/getServerInfo`).
    pub async fn get_server_info(&self) -> Result<Value> {
        let s = self.session.lock().await.clone();
        crate::protocol::auth::login::get_server_info(&s).await
    }

    // ------------------------------------------------------------------------
    // Friends & Profiles
    // ------------------------------------------------------------------------

    /// Fetch the user's friend list and 1-1 conversations (no local history is hydrated;
    /// use `social::friends::get_friends_with` to hydrate through a caller-supplied hook).
    pub async fn get_friends(&self) -> Result<Vec<Thread>> {
        let s = self.session.lock().await.clone();
        crate::protocol::social::friends::get_friends(&s).await
    }

    /// Batch-query mini profiles (display name, avatar, etc.) for a list of user IDs.
    pub async fn get_mini_profiles(
        &self,
        user_ids: &[String],
    ) -> Result<serde_json::Map<String, Value>> {
        let s = self.session.lock().await.clone();
        crate::protocol::social::groups::get_mini_profiles(&s, user_ids).await
    }

    /// Search for a Zalo user by raw phone number.
    pub async fn find_user_by_phone(&self, raw_phone: &str) -> Result<PhoneLookupResult> {
        let s = self.session.lock().await.clone();
        crate::protocol::social::friends::find_user_by_phone(&s, raw_phone).await
    }

    /// Query friendship status with a specific user.
    pub async fn check_friendship_status(&self, fid: &str) -> Result<FriendshipStatus> {
        let s = self.session.lock().await.clone();
        crate::protocol::social::friends::check_friendship_status(&s, fid).await
    }

    /// Send a friend request.
    pub async fn send_friend_request(&self, to_id: &str, msg: &str) -> Result<ActionAck> {
        let s = self.session.lock().await.clone();
        crate::protocol::social::friends::send_friend_request(&s, to_id, msg).await
    }

    /// Accept an incoming friend request.
    pub async fn accept_friend_request(&self, fid: &str) -> Result<ActionAck> {
        let s = self.session.lock().await.clone();
        crate::protocol::social::friends::accept_friend_request(&s, fid).await
    }

    /// Reject an incoming friend request.
    pub async fn reject_friend_request(&self, fid: &str) -> Result<ActionAck> {
        let s = self.session.lock().await.clone();
        crate::protocol::social::friends::reject_friend_request(&s, fid).await
    }

    // ------------------------------------------------------------------------
    // Groups
    // ------------------------------------------------------------------------

    /// Fetch all groups the current user belongs to.
    pub async fn get_groups(&self) -> Result<Vec<Thread>> {
        let s = self.session.lock().await.clone();
        crate::protocol::social::groups::get_groups(&s).await
    }

    /// Query profiles, names, and avatars for a list of group IDs.
    pub async fn fetch_group_profiles(
        &self,
        group_ids: &[String],
    ) -> Result<(
        serde_json::Map<String, Value>,
        std::collections::HashSet<String>,
    )> {
        let s = self.session.lock().await.clone();
        crate::protocol::social::groups::fetch_group_profiles(&s, group_ids).await
    }

    // ------------------------------------------------------------------------
    // Conversation History & Cloud Messages
    // ------------------------------------------------------------------------

    /// Get preview / last messages for given thread IDs.
    pub async fn get_last_messages(&self, thread_ids: &[String]) -> Result<LastMsgsBatch> {
        let s = self.session.lock().await.clone();
        crate::protocol::messaging::polling::get_last_msgs(&s, thread_ids).await
    }

    /// Fetch cloud messages (`/api/cm/getrecentv2` or `/api/cm/getoldv2`)
    /// supporting Groups, My Cloud, and 1-1 conversations.
    pub async fn get_cloud_messages(
        &self,
        conv_id: &str,
        global_msg_id: &str,
        is_old: bool,
        count: u32,
    ) -> Result<CloudMessagesPage> {
        let s = self.session.lock().await.clone();
        crate::protocol::messaging::polling::get_cloud_messages(
            &s,
            conv_id,
            global_msg_id,
            is_old,
            count,
        )
        .await
    }

    /// Fetch pinned conversations list.
    pub async fn get_pinned_conversations(&self) -> Result<Vec<String>> {
        let s = self.session.lock().await.clone();
        crate::protocol::messaging::threads::get_pinned(&s).await
    }

    /// Pin or unpin a conversation thread.
    pub async fn update_pinned_conversation(
        &self,
        thread_id: &str,
        is_group: bool,
        pin: bool,
    ) -> Result<ActionAck> {
        let s = self.session.lock().await.clone();
        crate::protocol::messaging::threads::update_pinned_conversation(
            &s, thread_id, is_group, pin,
        )
        .await
    }

    // ------------------------------------------------------------------------
    // Messaging & Sending
    // ------------------------------------------------------------------------

    /// Send a 1-1 text message.
    pub async fn send_direct_message(&self, to_id: &str, text: &str) -> Result<SendMessageReceipt> {
        let s = self.session.lock().await.clone();
        crate::protocol::messaging::send::send_message(&s, to_id, text).await
    }

    /// Send a group text message.
    pub async fn send_group_message(&self, grid: &str, text: &str) -> Result<SendMessageReceipt> {
        let s = self.session.lock().await.clone();
        crate::protocol::messaging::send::send_group_message(&s, grid, text).await
    }

    /// Send a quoted / reply message.
    pub async fn send_quote_message(
        &self,
        to_id: &str,
        text: &str,
        quoted: &ChatMessage,
        is_group: bool,
    ) -> Result<SendMessageReceipt> {
        let s = self.session.lock().await.clone();
        crate::protocol::messaging::send::send_quote_message(&s, to_id, text, quoted, is_group)
            .await
    }

    /// Send a link preview message.
    pub async fn send_link_message(
        &self,
        to_id: &str,
        text: &str,
        meta: &LinkMetadata,
        is_group: bool,
    ) -> Result<SendMessageReceipt> {
        let s = self.session.lock().await.clone();
        crate::protocol::messaging::links::send_link_message(&s, to_id, text, meta, is_group).await
    }

    /// Send a sticker.
    pub async fn send_sticker(
        &self,
        to_id: &str,
        sticker_id: u64,
        cate_id: u64,
        sticker_type: u32,
        is_group: bool,
    ) -> Result<SendMessageReceipt> {
        let s = self.session.lock().await.clone();
        crate::protocol::messaging::stickers::send_sticker(
            &s,
            to_id,
            sticker_id,
            cate_id,
            sticker_type,
            is_group,
        )
        .await
    }

    /// Send a file or photo attachment.
    pub async fn send_attachment(
        &self,
        to_id: &str,
        is_group: bool,
        file_path: &Path,
        file_name: &str,
    ) -> Result<ChatMessage> {
        let s = self.session.lock().await.clone();
        crate::protocol::messaging::attachments::send_attachment(
            &s, to_id, is_group, file_path, file_name,
        )
        .await
    }

    // ------------------------------------------------------------------------
    // Message Actions (Reaction, Delete, Recall, Forward)
    // ------------------------------------------------------------------------

    /// Send or update a reaction on a message.
    pub async fn send_reaction(
        &self,
        to_id: &str,
        is_group: bool,
        msg: &ChatMessage,
        reaction_icon: &str,
    ) -> Result<ActionAck> {
        let s = self.session.lock().await.clone();
        crate::protocol::messaging::reactions::send_reaction(
            &s,
            to_id,
            is_group,
            msg,
            reaction_icon,
        )
        .await
    }

    /// Delete a message for me.
    pub async fn delete_message_for_me(
        &self,
        to_id: &str,
        is_group: bool,
        msg: &ChatMessage,
    ) -> Result<ActionAck> {
        let s = self.session.lock().await.clone();
        crate::protocol::messaging::send::delete_message_for_me(&s, to_id, is_group, msg).await
    }

    /// Recall / undo an outgoing message for everyone.
    pub async fn recall_message(
        &self,
        to_id: &str,
        is_group: bool,
        msg: &ChatMessage,
    ) -> Result<ActionAck> {
        let s = self.session.lock().await.clone();
        crate::protocol::messaging::send::recall_message(&s, to_id, is_group, msg).await
    }

    // ------------------------------------------------------------------------
    // Polling & Real-Time Sync
    // ------------------------------------------------------------------------

    /// Long-poll for incoming messages (`lp_chat`) and merge them into `threads`.
    ///
    /// Nothing is persisted; callers store the result (`storage::persist_account_threads`).
    pub async fn poll_new_messages(
        &self,
        state: &mut PollState,
        threads: &mut Vec<Thread>,
    ) -> Result<()> {
        let s = self.session.lock().await.clone();
        crate::protocol::messaging::polling::poll_new_messages(&s, state, threads).await
    }

    /// Request mobile message history sync via wake-up protocol.
    pub async fn request_phone_sync(&self, public_key: &str) -> Result<ActionAck> {
        let s = self.session.lock().await.clone();
        crate::protocol::sync::request_phone_sync(&s, public_key).await
    }

    /// Clear pending sync state on mobile device.
    pub async fn clean_sync(&self, public_key: &str) -> Result<ActionAck> {
        let s = self.session.lock().await.clone();
        crate::protocol::sync::clean_sync(&s, public_key).await
    }

    /// Mark a conversation as unread or read.
    pub async fn set_unread_mark(
        &self,
        thread_id: &str,
        is_group: bool,
        mark_unread: bool,
    ) -> Result<ActionAck> {
        let s = self.session.lock().await.clone();
        crate::protocol::messaging::unread::set_unread_mark(&s, thread_id, is_group, mark_unread)
            .await
    }

    /// Mute or unmute notifications for a conversation.
    pub async fn set_mute_conversation(
        &self,
        thread_id: &str,
        is_group: bool,
        duration_sec: i64,
        mute: bool,
    ) -> Result<ActionAck> {
        let s = self.session.lock().await.clone();
        crate::protocol::messaging::unread::set_mute_conversation(
            &s,
            thread_id,
            is_group,
            duration_sec,
            mute,
        )
        .await
    }
    // ------------------------------------------------------------------------
    // Stickers
    // ------------------------------------------------------------------------

    /// Fetch sticker store categories and metadata.
    pub async fn fetch_sticker_store_packs(&self) -> Result<Vec<StickerPackInfo>> {
        let s = self.session.lock().await.clone();
        crate::protocol::messaging::stickers::fetch_sticker_store_packs(&s).await
    }

    /// Fetch stickers inside a specific category pack.
    pub async fn fetch_sticker_pack_items(
        &self,
        cate_id: u64,
        pack_name: &str,
    ) -> Result<Vec<StickerItem>> {
        let s = self.session.lock().await.clone();
        crate::protocol::messaging::stickers::fetch_sticker_pack_items(&s, cate_id, pack_name).await
    }

    /// Search stickers by Vietnamese keyword.
    pub async fn search_stickers_by_keyword(&self, keyword: &str) -> Result<Vec<StickerItem>> {
        let s = self.session.lock().await.clone();
        crate::protocol::messaging::stickers::search_stickers_by_keyword(&s, keyword).await
    }

    // ------------------------------------------------------------------------
    // Centralized Command Registry Dispatch
    // ------------------------------------------------------------------------

    /// Execute an authenticated `GET` for `spec` from [`crate::protocol::registry`].
    ///
    /// # Errors
    /// Returns an error if parameter encryption or candidate host execution fails.
    pub async fn api_get(
        &self,
        spec: crate::protocol::registry::CommandSpec,
        params: Option<&Value>,
    ) -> Result<Value> {
        let s = self.session.lock().await.clone();
        s.api_get(spec, params).await
    }

    /// Execute an authenticated `POST` for `spec` from [`crate::protocol::registry`].
    ///
    /// # Errors
    /// Returns an error if parameter encryption or candidate host execution fails.
    pub async fn api_post(
        &self,
        spec: crate::protocol::registry::CommandSpec,
        params: Option<&Value>,
    ) -> Result<Value> {
        let s = self.session.lock().await.clone();
        s.api_post(spec, params).await
    }

    /// Execute an authenticated `GET` by numeric `commandId` via [`crate::protocol::registry::lookup_command`].
    ///
    /// # Errors
    /// Returns an error if `command_id` is unknown or the request fails.
    pub async fn api_get_id(&self, command_id: u32, params: Option<&Value>) -> Result<Value> {
        let s = self.session.lock().await.clone();
        s.api_get_id(command_id, params).await
    }

    /// Execute an authenticated `POST` by numeric `commandId` via [`crate::protocol::registry::lookup_command`].
    ///
    /// # Errors
    /// Returns an error if `command_id` is unknown or the request fails.
    pub async fn api_post_id(&self, command_id: u32, params: Option<&Value>) -> Result<Value> {
        let s = self.session.lock().await.clone();
        s.api_post_id(command_id, params).await
    }
}
