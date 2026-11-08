//! Mention detection and classification (@all, direct mentions).

use serde_json::Value;

use crate::protocol::messaging::threads::ChatMessage;

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
    if ["uidFrom", "fromUid"]
        .iter()
        .any(|key| message.wire_ref.fields.get(*key).and_then(Value::as_str) == Some("0"))
    {
        return MentionDetection::NoMention;
    }
    let wire = message.wire_ref.fields.get("mentions");
    let archive = message
        .history_source
        .as_ref()
        .and_then(|source| source.pointer("/meta/mentionsList"));
    let Some(raw) = wire.or(archive) else {
        return MentionDetection::Unknown;
    };
    let Some(mentions) = raw.as_array() else {
        return MentionDetection::Unknown;
    };
    let mut unknown = false;
    for mention in mentions {
        let Some(uid) = mention.get("uid") else {
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
        if archive.is_some()
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
