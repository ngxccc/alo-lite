//! Mention detection and classification (@all, direct mentions).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::protocol::messaging::threads::ChatMessage;

use crate::protocol::wire::WireData;
/// Message envelope head carrying the sender id spellings that mark a system sender.
///
/// # Wire Response Example
/// ```json
/// { "uidFrom": "0", "fromUid": "0" }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MentionSenderWireData {
    #[serde(rename = "uidFrom")]
    pub uid_from: Option<Value>,
    #[serde(rename = "fromUid")]
    pub from_uid: Option<Value>,
}

impl MentionSenderWireData {
    /// Build from an already-decoded wire-field map (`WireMessageRef::fields`).
    pub fn from_fields(fields: &BTreeMap<String, Value>) -> Self {
        Self {
            uid_from: fields.get("uidFrom").cloned(),
            from_uid: fields.get("fromUid").cloned(),
        }
    }

    /// Whether the sender is the literal `"0"` system marker (never a mention target).
    pub fn is_system_sender(&self) -> bool {
        [self.uid_from.as_ref(), self.from_uid.as_ref()]
            .into_iter()
            .any(|v| v.and_then(Value::as_str) == Some("0"))
    }
}

/// Archive history source carrying `meta.mentionsList`.
///
/// # Wire Response Example
/// ```json
/// { "meta": { "mentionsList": [ { "uid": "1234567890" } ] } }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HistoryMentionsWireData {
    #[serde(rename = "meta")]
    pub meta: Option<Value>,
}

impl HistoryMentionsWireData {
    /// The `mentionsList` array, when present.
    pub fn mentions_list(&self) -> Option<&Vec<Value>> {
        self.meta.as_ref()?.get("mentionsList")?.as_array()
    }
}

/// One entry of a message `mentions` list.
///
/// # Wire Response Example
/// ```json
/// { "uid": "1234567890", "pos": 0, "len": 3 }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MentionWireData {
    #[serde(rename = "uid")]
    pub uid: Option<Value>,
}

impl MentionWireData {
    /// Raw `uid` value, or `None` when the entry carries no uid at all.
    pub fn uid_value(&self) -> Option<&Value> {
        self.uid.as_ref()
    }
}

/// Unknown is distinct from a verified empty/nonmatching raw mentions array.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MentionDetection {
    Mention,
    NoMention,
    Unknown,
}

/// Verified original desktop MENTION_ALL_ID, not a display-name/text heuristic.
pub const MENTION_ALL_ID: &str = "-1";

pub fn detect_message_mention(message: &ChatMessage, account: &str) -> bool {
    classify_message_mention(message, account) == MentionDetection::Mention
}

pub fn classify_message_mention(message: &ChatMessage, account: &str) -> MentionDetection {
    if account.is_empty() || account == "0" {
        return MentionDetection::Unknown;
    }
    if message.is_outgoing {
        return MentionDetection::NoMention;
    }
    // Original lazy/default-login-main-startup-shared-worker-znotification.d914...:
    // U(e) tests mentions[].uid == session UID || uid === MENTION_ALL_ID; ji="-1".
    // Original shared-worker.06caa1ffa528545b8f43.js syncMessage2SVMessage:
    // meta.mentionsList becomes mentions; opaque UIDs are decoded, while "-1" stays.
    if MentionSenderWireData::from_fields(&message.wire_ref.fields).is_system_sender() {
        return MentionDetection::NoMention;
    }
    let wire = message.wire_ref.fields.get("mentions").cloned();
    let archive = message
        .history_source
        .as_ref()
        .and_then(|source| {
            HistoryMentionsWireData::from_value(source)
                .mentions_list()
                .cloned()
        })
        .map(Value::Array);
    let from_archive = archive.is_some();
    let Some(raw) = wire.or(archive) else {
        return MentionDetection::Unknown;
    };
    let Some(mentions) = raw.as_array() else {
        return MentionDetection::Unknown;
    };
    let mut unknown = false;
    for mention in mentions {
        let mention = MentionWireData::from_value(mention);
        let Some(uid) = mention.uid_value() else {
            unknown = true;
            continue;
        };
        let target = uid
            .as_str()
            .map(str::to_owned)
            .or_else(|| uid.as_u64().map(|uid| uid.to_string()));
        if uid.as_str() == Some(MENTION_ALL_ID) || target.as_deref() == Some(account) {
            return MentionDetection::Mention;
        }
        if target.is_none() {
            unknown = true;
        }
        // An archive noised UID that does not exactly match the account is not a
        // verified negative without the original decoder's identity mapping.
        if from_archive
            && target
                .as_ref()
                .is_some_and(|uid| !uid.bytes().all(|b| b.is_ascii_digit()))
        {
            unknown = true;
        }
    }
    if unknown {
        MentionDetection::Unknown
    } else {
        MentionDetection::NoMention
    }
}
