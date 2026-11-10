//! Message domain entity (`ChatMessage`), metadata merging, reaction ledger, and media hydration.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::endpoints;
use crate::protocol::messaging::attachments::extract_link_info;
use crate::protocol::messaging::model::{DeliveryState, MessageKind, UnreadPolicy, WireMessageRef};
use crate::protocol::messaging::reactions::{
    legacy_emojis, merge_untracked_snapshot, parse_reactions, ReactionEvent,
    ReactionSnapshotProvenance, ReactionSummary, ReactionVersion,
};
use crate::protocol::wire::{val_i64, WireData};

#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type)]
pub struct ChatMessage {
    #[serde(default)]
    pub kind: MessageKind,
    #[serde(default)]
    pub wire_ref: WireMessageRef,
    #[serde(default)]
    pub unread_policy: UnreadPolicy,
    #[serde(default)]
    pub delivery: DeliveryState,
    pub id: String,
    pub sender: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_id: Option<String>,
    pub text: String,
    pub time: String,
    pub is_outgoing: bool,
    pub ts: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sticker_id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sticker_cate_id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sticker_type: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to_sender: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to_text: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reactions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reaction_summaries: Vec<ReactionSummary>,
    #[serde(default)]
    pub reaction_snapshot: bool,
    #[serde(default)]
    pub reaction_authoritative_snapshot: bool,
    #[serde(skip)]
    pub reaction_snapshot_fresh: bool,
    #[serde(default)]
    pub reaction_snapshot_provenance: ReactionSnapshotProvenance,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub reaction_versions: BTreeMap<String, ReactionVersion>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub client_message_id: String,
    #[serde(default)]
    pub is_pinned: bool,
    #[serde(default)]
    pub is_starred: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_desc: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_thumb: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_src: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history_source: Option<Value>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub resolved_mention_uids: BTreeMap<String, String>,
}

/// Archive history source carrying media metadata (`msgType`, `meta.attachsList`).
///
/// # Wire Response Example
/// ```json
/// { "msgType": 2, "meta": { "attachsList": [ { "href": "https://photo.example/1" } ] } }
/// ```
///
/// # Core Transformation
/// Yields the source-derived media label and the first attachment object that
/// `extract_link_info` turns into link metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HistoryMediaWireData {
    #[serde(rename = "msgType")]
    pub msg_type: Option<Value>,
    #[serde(rename = "meta")]
    pub meta: Option<Value>,
}

impl HistoryMediaWireData {
    /// Source-derived media label for Zalo photo (`2..=4`) and video (`19`) message types.
    pub fn source_kind(&self) -> Option<&'static str> {
        match val_i64(self.msg_type.as_ref()) {
            Some(2..=4) => Some("Hình ảnh Zalo"),
            Some(19) => Some("Video Zalo"),
            _ => None,
        }
    }

    /// First entry of `meta.attachsList`.
    pub fn first_attachment(&self) -> Option<&Value> {
        self.meta.as_ref()?.get("attachsList")?.as_array()?.first()
    }
}

impl ChatMessage {
    pub fn merge_message_metadata(&mut self, incoming: &Self) -> bool {
        let previous_mentions = self.resolved_mention_uids.clone();
        self.resolved_mention_uids
            .extend(incoming.resolved_mention_uids.clone());
        let before = (
            self.kind,
            self.wire_ref.clone(),
            self.unread_policy.clone(),
            self.delivery,
            self.history_source.clone(),
        );
        if incoming.kind != MessageKind::Unknown {
            self.kind = incoming.kind;
        }
        if !incoming.wire_ref.fields.is_empty() {
            let mut fields = self.wire_ref.fields.clone();
            fields.extend(incoming.wire_ref.fields.clone());
            self.wire_ref = WireMessageRef::from_raw(&Value::Object(fields.into_iter().collect()));
            if incoming.unread_policy.params_ext_present {
                self.unread_policy = incoming.unread_policy.clone();
            }
        }
        if incoming.history_source.is_some() {
            self.history_source = incoming.history_source.clone();
        }
        self.delivery = self.delivery.merge(incoming.delivery);
        previous_mentions != self.resolved_mention_uids
            || before
                != (
                    self.kind,
                    self.wire_ref.clone(),
                    self.unread_policy.clone(),
                    self.delivery,
                    self.history_source.clone(),
                )
    }

    /// Keep aggregate counts and event replay cursors across cache/network merges.
    pub fn merge_reaction_metadata(&mut self, incoming: &Self) -> bool {
        let model_changed = self.merge_message_metadata(incoming);
        let previous_provenance = self.reaction_snapshot_provenance.clone();
        let before = (
            self.reaction_summaries.clone(),
            self.reactions.clone(),
            self.client_message_id.clone(),
            self.reaction_snapshot,
            self.reaction_authoritative_snapshot,
            self.reaction_versions.clone(),
        );
        if self.client_message_id.is_empty() {
            self.client_message_id = incoming.client_message_id.clone();
        }
        let accept_snapshot = incoming.reaction_authoritative_snapshot
            && (incoming.reaction_snapshot_fresh || !self.reaction_authoritative_snapshot)
            && self.reaction_snapshot_provenance.accepts_snapshot(
                &incoming.reaction_snapshot_provenance,
                &self.reaction_versions,
                incoming.reaction_snapshot_fresh,
            );

        if accept_snapshot {
            self.reaction_snapshot_provenance = incoming.reaction_snapshot_provenance.clone();
            self.reaction_summaries = incoming.reaction_summaries.clone();
            self.reaction_authoritative_snapshot = true;
            self.reaction_snapshot = true;
            for (user, version) in &mut self.reaction_versions {
                version.covered_events.append(&mut version.events);
                version.baseline = incoming
                    .reaction_summaries
                    .iter()
                    .filter_map(|s| Some((s.r_type?, *s.reactor_counts.get(user)?)))
                    .collect();
                version.ledger_initialized = true;
                version.snapshot_baseline = true;
            }
            for (user, incoming_version) in &incoming.reaction_versions {
                let covered = self
                    .reaction_versions
                    .get(user)
                    .map(|v| v.covered_events.clone())
                    .unwrap_or_default();
                let mut version = incoming_version.clone();
                version.covered_events.extend(covered);
                if incoming.reaction_snapshot_fresh {
                    version.covered_events.append(&mut version.events);
                    version.baseline = incoming
                        .reaction_summaries
                        .iter()
                        .filter_map(|s| Some((s.r_type?, *s.reactor_counts.get(user)?)))
                        .collect();
                    version.ledger_initialized = true;
                    version.snapshot_baseline = true;
                }
                self.reaction_versions.insert(user.clone(), version);
            }
        }
        if !self.reaction_authoritative_snapshot
            && !incoming.reaction_authoritative_snapshot
            && (incoming.reaction_snapshot || !incoming.reaction_summaries.is_empty())
            && self.reaction_versions.is_empty()
        {
            self.reaction_summaries = incoming.reaction_summaries.clone();
            self.reactions = legacy_emojis(&self.reaction_summaries);
            self.reaction_snapshot = incoming.reaction_snapshot;
        }
        if !self.reaction_authoritative_snapshot
            && !incoming.reaction_authoritative_snapshot
            && !self.reaction_versions.is_empty()
            && (incoming.reaction_snapshot || !incoming.reaction_summaries.is_empty())
        {
            merge_untracked_snapshot(
                &mut self.reaction_summaries,
                &incoming.reaction_summaries,
                &self.reaction_versions,
                incoming.reaction_snapshot && incoming.reaction_versions.is_empty(),
            );
        }
        for (user, version) in &incoming.reaction_versions {
            if accept_snapshot {
                continue;
            }
            if !self.reaction_versions.contains_key(user) {
                if self.reaction_authoritative_snapshot {
                    for event in &version.events {
                        if event.r_type >= -1 {
                            self.apply_reaction(event);
                        }
                    }
                    continue;
                }
                self.reaction_versions.insert(user.clone(), version.clone());
                if version.ledger_initialized {
                    version.rebuild(&mut self.reaction_summaries, user);
                }
            } else {
                for event in &version.events {
                    if event.r_type >= -1 {
                        self.apply_reaction(event);
                    }
                }
            }
        }
        if !self.reaction_summaries.is_empty()
            || incoming.reaction_snapshot
            || !incoming.reaction_versions.is_empty()
        {
            self.reactions = legacy_emojis(&self.reaction_summaries);
        } else if self.reactions.is_empty()
            && self.reaction_versions.is_empty()
            && !self.reaction_snapshot
        {
            self.reactions = incoming.reactions.clone();
        }
        model_changed
            || previous_provenance != self.reaction_snapshot_provenance
            || before
                != (
                    self.reaction_summaries.clone(),
                    self.reactions.clone(),
                    self.client_message_id.clone(),
                    self.reaction_snapshot,
                    self.reaction_authoritative_snapshot,
                    self.reaction_versions.clone(),
                )
    }

    pub fn apply_reaction(&mut self, event: &ReactionEvent) -> bool {
        if event.sender_id.is_empty() || event.r_type < -1 {
            return false;
        }
        if self.reaction_summaries.is_empty() && !self.reactions.is_empty() {
            self.reaction_summaries = parse_reactions(&json!({"reactions": self.reactions}));
        }
        let changed = self
            .reaction_versions
            .entry(event.sender_id.clone())
            .or_default()
            .record(
                &mut self.reaction_summaries,
                event,
                self.reaction_authoritative_snapshot,
            );
        self.reactions = legacy_emojis(&self.reaction_summaries);
        self.reaction_snapshot = true;
        changed
    }

    /// Repair cached media metadata without changing captions or signed URL bytes.
    pub fn hydrate_media_metadata(&mut self) -> bool {
        if self.text == "[Tin nhắn đã thu hồi]" {
            return false;
        }
        let before = (
            self.link_url.clone(),
            self.link_thumb.clone(),
            self.link_src.clone(),
        );
        let media = self
            .history_source
            .as_ref()
            .map(HistoryMediaWireData::from_value);
        let source_kind = media.as_ref().and_then(HistoryMediaWireData::source_kind);
        if let Some(media) = &media {
            if let Some(attachment) = media.first_attachment() {
                let (url, _, _, thumb, _) = extract_link_info(attachment);
                if self.link_url.is_none() {
                    self.link_url = url;
                }
                if self.link_thumb.is_none() {
                    self.link_thumb = thumb;
                }
            }
        }
        let label_kind = match self.text.trim() {
            "[Photo]" | "[Hình ảnh]" => Some("Hình ảnh Zalo"),
            "[Video]" => Some("Video Zalo"),
            _ => None,
        };
        if let Some(kind) = source_kind.or(label_kind) {
            self.link_src = Some(kind.into());
            if kind == "Hình ảnh Zalo" && self.link_thumb.is_none() {
                self.link_thumb = self.link_url.clone();
            }
        }
        before
            != (
                self.link_url.clone(),
                self.link_thumb.clone(),
                self.link_src.clone(),
            )
    }

    /// Reconcile an incoming duplicate/echo message into `self` without losing metadata.
    pub(crate) fn reconcile_incoming(&mut self, msg: ChatMessage, overwrite_reactions: bool) {
        self.merge_reaction_metadata(&msg);
        if self.id.starts_with("out_") && !msg.id.is_empty() && !msg.id.starts_with("out_") {
            self.id = msg.id.clone();
        }
        if msg.ts > 0 && self.ts != msg.ts {
            self.ts = msg.ts;
            self.time = msg.time;
        }
        if msg.text.trim() == "[Tin nhắn đã thu hồi]" {
            self.text = "[Tin nhắn đã thu hồi]".to_string();
            self.sticker_id = None;
            self.link_url = None;
            self.link_thumb = None;
        }
        if self.sticker_id.is_none() && msg.sticker_id.is_some() {
            self.sticker_id = msg.sticker_id;
            self.sticker_cate_id = msg.sticker_cate_id;
            self.sticker_type = msg.sticker_type;
        }
        if self.reply_to_text.is_none() && msg.reply_to_text.is_some() {
            self.reply_to_id = msg.reply_to_id;
            self.reply_to_sender = msg.reply_to_sender;
            self.reply_to_text = msg.reply_to_text;
        }
        if self.link_url.is_none() && msg.link_url.is_some() {
            self.link_url = msg.link_url;
            self.link_title = msg.link_title;
            self.link_desc = msg.link_desc;
            self.link_thumb = msg.link_thumb;
            self.link_src = msg.link_src;
            if !msg.text.is_empty() {
                self.text = msg.text;
            }
        }
        if !msg.reactions.is_empty() && (overwrite_reactions || self.reactions.is_empty()) {
            self.reactions = msg.reactions;
        }
        self.is_pinned = self.is_pinned || msg.is_pinned;
        self.is_starred = self.is_starred || msg.is_starred;
    }

    /// Compute the static PNG URL for a Zalo sticker.
    pub fn sticker_image_url(&self) -> Option<String> {
        let sid = self.sticker_id?;
        if sid == 0 {
            return None;
        }
        if self.sticker_cate_id == Some(0) && (1..=30).contains(&sid) {
            Some(format!(
                "{}/{sid}.png",
                endpoints::STICKER_DEFAULT_THUMB_BASE
            ))
        } else {
            Some(format!(
                "{}?eid={sid}&size=130",
                endpoints::STICKER_WEBPC_BASE
            ))
        }
    }

    pub fn sticker_cache_key(&self) -> Option<String> {
        let sid = self.sticker_id?;
        if sid == 0 {
            return None;
        }
        let cid = self.sticker_cate_id.unwrap_or(0);
        Some(format!("stk_{cid}_{sid}"))
    }
}
