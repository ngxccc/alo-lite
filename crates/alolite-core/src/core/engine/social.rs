//! Friend relationship checking, friend requests, and phone lookup.

use super::CoreEngine;
use crate::core::events::CoreEvent;
use crate::protocol::social::friends::{
    accept_friend_request, check_friendship_status, find_user_by_phone, get_user_online_status,
    reject_friend_request, send_friend_request, undo_friend_request, FriendshipStatus,
};

impl CoreEngine {
    pub async fn check_friend_status(&self, user_id: String) {
        let s = self.session.lock().await.clone();
        let status_res = check_friendship_status(&s, &user_id).await;
        let online_opt = get_user_online_status(&s, &user_id).await.ok();
        if let Ok(status) = status_res {
            self.emit(CoreEvent::FriendStatusChecked {
                user_id,
                status,
                online: online_opt,
            });
        }
    }

    pub async fn accept_friend(&self, user_id: String, user_name: String) {
        let s = self.session.lock().await.clone();
        match accept_friend_request(&s, &user_id).await {
            Ok(_) => {
                self.emit(CoreEvent::FriendStatusChecked {
                    user_id,
                    status: FriendshipStatus::Friend,
                    online: None,
                });
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Đã đồng ý kết bạn với {user_name}!"
                )));
            }
            Err(e) => {
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Không thể đồng ý kết bạn với {user_name}: {e}"
                )));
            }
        }
    }

    pub async fn reject_friend(&self, user_id: String, user_name: String) {
        let s = self.session.lock().await.clone();
        if reject_friend_request(&s, &user_id).await.is_ok() {
            self.emit(CoreEvent::FriendStatusChecked {
                user_id,
                status: FriendshipStatus::Stranger,
                online: None,
            });
            self.emit(CoreEvent::StatusUpdate(format!(
                "Đã từ chối lời mời kết bạn của {user_name}."
            )));
        }
    }

    pub async fn undo_friend(&self, user_id: String, user_name: String) {
        let s = self.session.lock().await.clone();
        if undo_friend_request(&s, &user_id).await.is_ok() {
            self.emit(CoreEvent::FriendStatusChecked {
                user_id,
                status: FriendshipStatus::Stranger,
                online: None,
            });
            self.emit(CoreEvent::StatusUpdate(format!(
                "Đã hủy lời mời kết bạn tới {user_name}."
            )));
        }
    }

    pub async fn add_friend(&self, user_id: String, user_name: String) {
        let s = self.session.lock().await.clone();
        if accept_friend_request(&s, &user_id).await.is_ok() {
            self.emit(CoreEvent::FriendStatusChecked {
                user_id,
                status: FriendshipStatus::Friend,
                online: None,
            });
            self.emit(CoreEvent::StatusUpdate(format!(
                "Đã đồng ý kết bạn với {user_name}!"
            )));
            return;
        }
        match send_friend_request(&s, &user_id, "Xin chào, kết bạn với mình nhé!").await {
            Ok(_) => {
                self.emit(CoreEvent::FriendStatusChecked {
                    user_id: user_id.clone(),
                    status: FriendshipStatus::OutgoingPending,
                    online: None,
                });
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Đã gửi lời mời kết bạn tới {user_name} ({user_id})."
                )));
            }
            Err(e) => {
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Không thể gửi lời mời kết bạn tới {user_name}: {e}"
                )));
            }
        }
    }

    pub async fn lookup_phone(&self, phone: String) {
        let s = self.session.lock().await.clone();
        match find_user_by_phone(&s, &phone).await {
            Ok(profile) => self.emit(CoreEvent::PhoneLookupSuccess(profile)),
            Err(e) => {
                crate::logger::log_error("core::lookup_phone", &e.to_string());
                self.emit(CoreEvent::PhoneLookupFailed(e.to_string()));
            }
        }
    }
}
