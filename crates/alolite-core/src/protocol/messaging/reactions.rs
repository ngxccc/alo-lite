//! Reaction summaries and wire events, preserving counts and lossless message IDs.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::protocol::wire::WireData;
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct ReactionSummary {
    pub emoji: String,
    pub count: u64,
    pub reactor_ids: Vec<String>,
    pub r_type: Option<i32>,
    pub count_is_lower_bound: bool,
    pub reactor_counts: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct ReactionEvent {
    pub message_id: String,
    pub client_message_id: String,
    pub thread_id: String,
    pub sender_id: String,
    pub event_id: String,
    pub client_event_id: String,
    pub r_type: i32,
    pub emoji: String,
    pub ts: i64,
    pub historical: bool,
}

/// Persisted per-reactor ledger. IDs deduplicate replay; chronological replay handles old pages.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct ReactionVersion {
    pub ts: i64,
    pub event_id: String,
    pub client_event_id: String,
    pub events: Vec<ReactionEvent>,
    pub covered_events: Vec<ReactionEvent>,
    pub baseline: BTreeMap<i32, u64>,
    pub ledger_initialized: bool,
    pub snapshot_baseline: bool,
}
fn known_id(id: &str) -> bool {
    !id.is_empty() && id != "0"
}
fn id_order(a: &str, b: &str) -> std::cmp::Ordering {
    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}
fn event_order(a: &ReactionEvent, b: &ReactionEvent) -> std::cmp::Ordering {
    crate::clock::normalize_ts_ms(a.ts)
        .cmp(&crate::clock::normalize_ts_ms(b.ts))
        .then_with(|| id_order(&a.client_event_id, &b.client_event_id))
        .then_with(|| id_order(&a.event_id, &b.event_id))
        // Without ordering IDs a clear wins an indistinguishable timestamp.
        .then_with(|| (a.r_type == -1).cmp(&(b.r_type == -1)))
}
fn same_event(a: &ReactionEvent, b: &ReactionEvent) -> bool {
    if known_id(&a.event_id) && known_id(&b.event_id) && a.event_id == b.event_id {
        return true;
    }
    if known_id(&a.client_event_id)
        && known_id(&b.client_event_id)
        && a.client_event_id == b.client_event_id
    {
        return true;
    }
    if known_id(&a.event_id)
        || known_id(&b.event_id)
        || known_id(&a.client_event_id)
        || known_id(&b.client_event_id)
    {
        return false;
    }
    a.ts > 0
        && b.ts > 0
        && crate::clock::normalize_ts_ms(a.ts) == crate::clock::normalize_ts_ms(b.ts)
        && a.r_type == b.r_type
}
impl From<&ReactionEvent> for ReactionVersion {
    fn from(event: &ReactionEvent) -> Self {
        Self {
            ts: crate::clock::normalize_ts_ms(event.ts),
            event_id: event.event_id.clone(),
            client_event_id: event.client_event_id.clone(),
            ..Default::default()
        }
    }
}
impl ReactionVersion {
    pub fn precedes(&self, event: &ReactionEvent) -> bool {
        event_order(
            &ReactionEvent {
                ts: self.ts,
                event_id: self.event_id.clone(),
                client_event_id: self.client_event_id.clone(),
                ..Default::default()
            },
            event,
        )
        .is_lt()
    }

    pub fn record(
        &mut self,
        summaries: &mut Vec<ReactionSummary>,
        event: &ReactionEvent,
        authoritative: bool,
    ) -> bool {
        if !self.ledger_initialized {
            for summary in summaries.iter() {
                if let (Some(kind), Some(n)) =
                    (summary.r_type, summary.reactor_counts.get(&event.sender_id))
                {
                    self.baseline.insert(kind, *n);
                }
            }
            // Old cache cursors have no event type. Retain their known identity without guessing one.
            if known_id(&self.event_id) || known_id(&self.client_event_id) {
                self.events.push(ReactionEvent {
                    ts: self.ts,
                    event_id: self.event_id.clone(),
                    client_event_id: self.client_event_id.clone(),
                    r_type: -2,
                    sender_id: event.sender_id.clone(),
                    ..Default::default()
                });
            }
            self.ledger_initialized = true;
            self.snapshot_baseline = authoritative;
        }
        if self
            .events
            .iter()
            .chain(&self.covered_events)
            .any(|seen| same_event(seen, event))
        {
            return false;
        }
        self.events.push(event.clone());
        self.events.sort_by(event_order);
        if self.precedes(event) {
            self.ts = crate::clock::normalize_ts_ms(event.ts);
            self.event_id = event.event_id.clone();
            self.client_event_id = event.client_event_id.clone();
        }
        self.rebuild(summaries, &event.sender_id);
        true
    }

    pub fn rebuild(&self, summaries: &mut Vec<ReactionSummary>, sender: &str) {
        apply_reaction_event(
            summaries,
            &ReactionEvent {
                sender_id: sender.into(),
                r_type: -1,
                ..Default::default()
            },
        );
        for (kind, n) in &self.baseline {
            add(
                summaries,
                Some(*kind),
                emoji_for_type(*kind).into(),
                *n,
                false,
                Some(sender),
            );
        }
        for event in &self.events {
            if event.r_type >= -1 && !(self.snapshot_baseline && event.historical) {
                apply_reaction_event(summaries, event);
            }
        }
    }
}

pub fn emoji_for_type(r_type: i32) -> &'static str {
    match r_type {
        3 => "👍",
        5 => "❤️",
        0 => "😆",
        32 => "😮",
        2 => "😢",
        20 => "😠",
        46 => "👏",
        120 => "🌹",
        65 => "💔",
        4 => "👎",
        68 => "👌",
        69 => "✌️",
        70 => "🙏",
        66 => "💩",
        101 => "🎉",
        _ => "◦",
    }
}

pub fn type_for_icon(icon: &str) -> Option<i32> {
    match icon.trim() {
        "/-strong" | "👍" => Some(3),
        "/-heart" | "❤" | "❤️" => Some(5),
        ":>" | "😆" => Some(0),
        ":o" | "😮" => Some(32),
        ":-((" | ":-(( " | "😢" => Some(2),
        ":-h" | "😠" => Some(20),
        ":handclap" | "👏" => Some(46),
        "/-rose" | "🌹" => Some(120),
        "/-break" | "💔" => Some(65),
        "/-weak" | "👎" => Some(4),
        "/-ok" | "👌" => Some(68),
        "/-v" | "✌️" | "✌" => Some(69),
        "/-thanks" | "🙏" => Some(70),
        "/-shit" | "💩" => Some(66),
        "🎉" => Some(101),
        _ => None,
    }
}

fn id(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

fn count(value: &Value) -> Option<(u64, bool)> {
    if value.as_str() == Some("9k+") {
        return Some((9001, true));
    }
    value
        .as_u64()
        .or_else(|| value.as_str()?.parse().ok())
        .filter(|n| *n > 0)
        .map(|n| (n, false))
}

fn add(
    out: &mut Vec<ReactionSummary>,
    r_type: Option<i32>,
    emoji: String,
    n: u64,
    lower: bool,
    user: Option<&str>,
) {
    let index = out
        .iter()
        .position(|r| r.r_type == r_type && r.emoji == emoji);
    let i = index.unwrap_or_else(|| {
        out.push(ReactionSummary {
            emoji,
            r_type,
            ..Default::default()
        });
        out.len() - 1
    });
    let summary = &mut out[i];
    summary.count = summary.count.saturating_add(n);
    summary.count_is_lower_bound |= lower;
    if let Some(user) = user.filter(|s| !s.is_empty()) {
        if !summary.reactor_ids.iter().any(|s| s == user) {
            summary.reactor_ids.push(user.into());
        }
        let c = summary.reactor_counts.entry(user.into()).or_default();
        *c = c.saturating_add(n);
    }
}

/// Legacy reaction list entry (`reacts: [ { "rIcon": "/-heart", "count": 2 } ]`).
///
/// # Wire Response Example
/// ```json
/// { "rIcon": "/-heart", "count": 2 }
/// ```
///
/// # Core Transformation
/// The legacy shape encoded unique icons with an optional count, never exact multiplicity.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LegacyReactionWireData {
    #[serde(rename = "rIcon")]
    pub r_icon: Option<Value>,
    #[serde(rename = "count")]
    pub count: Option<Value>,
}

impl LegacyReactionWireData {
    /// Reaction icon (a `/‑strong`-style token or an emoji).
    pub fn icon(&self) -> Option<&str> {
        self.r_icon.as_ref().and_then(Value::as_str)
    }

    /// Shipped count; absent means a single reaction.
    pub fn count(&self) -> (u64, bool) {
        self.count.as_ref().and_then(count).unwrap_or((1, false))
    }
}

/// Target message of a reaction (`content.rMsg` entry).
///
/// # Wire Response Example
/// ```json
/// { "gMsgID": "1731215399000_9", "cMsgID": "8888" }
/// ```
///
/// # Core Transformation
/// Both ids are kept because either may be the only one the local store can match.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ReactionTargetWireData {
    #[serde(rename = "gMsgID")]
    pub global_msg_id: Option<Value>,
    #[serde(rename = "cMsgID")]
    pub client_msg_id: Option<Value>,
}

impl ReactionTargetWireData {
    /// Server-side message id (`gMsgID`).
    pub fn message_id(&self) -> String {
        id(self.global_msg_id.as_ref())
    }

    /// Client-side message id (`cMsgID`).
    pub fn client_message_id(&self) -> String {
        id(self.client_msg_id.as_ref())
    }
}

/// Reaction content (`content`, or `content.params` when nested).
///
/// # Wire Response Example
/// ```json
/// { "rType": 5, "rMsg": [ { "gMsgID": "1731215399000_9" } ] }
/// ```
///
/// # Core Transformation
/// `rType == -1` is a clear; every other non-negative value is an add.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ReactionContentWireData {
    #[serde(rename = "rType")]
    pub r_type: Option<Value>,
    #[serde(rename = "rMsg")]
    pub r_msg: Option<Vec<Value>>,
    #[serde(rename = "rIcon")]
    pub r_icon: Option<Value>,
}

impl ReactionContentWireData {
    /// Reaction type; `None` when absent, non-numeric, or outside `i32`.
    pub fn r_type(&self) -> Option<i32> {
        self.r_type
            .as_ref()
            .and_then(Value::as_i64)
            .and_then(|n| i32::try_from(n).ok())
    }

    /// Raw target entries.
    pub fn targets(&self) -> &[Value] {
        self.r_msg.as_deref().unwrap_or_default()
    }

    /// Reaction icon; `None` when absent or non-string.
    pub fn r_icon(&self) -> Option<&str> {
        self.r_icon.as_ref().and_then(Value::as_str)
    }
}

/// Socket `chat.reaction` envelope, or one item of a reaction batch.
///
/// # Wire Response Example
/// ```json
/// {
///   "uidFrom": "1234567890",
///   "msgKey": "1234567890",
///   "msgId": "1731215400000_1",
///   "cliMsgId": "9999",
///   "ts": 1731215400000,
///   "content": { "rType": 3, "rMsg": [ { "gMsgID": "1731215399000_9" } ] }
/// }
/// ```
///
/// # Core Transformation
/// Resolves the thread id (nested `content.params` selects `msgKey`, otherwise `idTo`),
/// requires a sender and at least one target, and emits one [`ReactionEvent`] per target.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ReactionEventWireData {
    #[serde(rename = "uidFrom")]
    pub uid_from: Option<Value>,
    #[serde(rename = "msgKey")]
    pub msg_key: Option<Value>,
    #[serde(rename = "idTo")]
    pub id_to: Option<Value>,
    #[serde(rename = "msgId")]
    pub msg_id: Option<Value>,
    #[serde(rename = "cliMsgId")]
    pub cli_msg_id: Option<Value>,
    #[serde(rename = "ts")]
    pub ts: Option<Value>,
    #[serde(rename = "content")]
    pub content: Option<Value>,
    #[serde(rename = "reacts")]
    pub reacts: Option<Vec<Value>>,
    #[serde(rename = "reactGroups")]
    pub react_groups: Option<Vec<Value>>,
    #[serde(rename = "reactions")]
    pub reactions: Option<Vec<Value>>,
    #[serde(rename = "messages")]
    pub messages: Option<Vec<Value>>,
}

impl ReactionEventWireData {
    /// Sender id (`uidFrom`), empty when absent.
    pub fn sender_id(&self) -> String {
        id(self.uid_from.as_ref())
    }

    /// Event timestamp accepting a number or a numeric string.
    pub fn timestamp(&self) -> i64 {
        self.ts
            .as_ref()
            .and_then(|v| v.as_i64().or_else(|| v.as_str()?.trim().parse().ok()))
            .unwrap_or(0)
    }

    /// Thread id: `msgKey` when `content` is nested under `params`, else `idTo`.
    pub fn thread_id(&self) -> String {
        if self.has_nested_params() {
            id(self.msg_key.as_ref())
        } else {
            id(self.id_to.as_ref())
        }
    }

    /// Whether `content.params` exists (the nested-envelope form).
    pub fn has_nested_params(&self) -> bool {
        self.content
            .as_ref()
            .and_then(|c| c.get("params"))
            .is_some()
    }

    /// Reaction content, decoded from a JSON string when the wire sends one.
    pub fn reaction_content(&self) -> Option<ReactionContentWireData> {
        let outer = self.content.as_ref()?;
        let value = outer.get("params").unwrap_or(outer);
        let value = match value {
            Value::String(s) => serde_json::from_str::<Value>(s).ok()?,
            other => other.clone(),
        };
        Some(ReactionContentWireData::from_value(&value))
    }
}

/// Parse shipped per-user/per-type counts, plus the application's legacy icon list.
/// An absent/null payload is not an authoritative clear; callers must preserve that distinction.
pub fn parse_reactions(value: &Value) -> Vec<ReactionSummary> {
    let root = value.get("reacts").unwrap_or(value);
    let payload = root.get("reactions").unwrap_or(root);
    let mut out = Vec::new();
    if let Some(users) = payload.as_object() {
        for (key, val) in users {
            if let Some(types) = val.as_object() {
                for (kind, v) in types {
                    let Ok(kind) = kind.parse::<i32>() else {
                        continue;
                    };
                    if kind < 0 {
                        continue;
                    }
                    if let Some((n, lower)) = count(v) {
                        add(
                            &mut out,
                            Some(kind),
                            emoji_for_type(kind).into(),
                            n,
                            lower,
                            Some(key),
                        );
                    }
                }
            } else if let Ok(kind) = key.parse::<i32>() {
                if kind >= 0 {
                    if let Some((n, lower)) = count(val) {
                        add(
                            &mut out,
                            Some(kind),
                            emoji_for_type(kind).into(),
                            n,
                            lower,
                            None,
                        );
                    }
                }
            }
        }
    } else if let Some(items) = payload.as_array() {
        for item in items {
            let wire = LegacyReactionWireData::from_value(item);
            let icon = item.as_str().or_else(|| wire.icon());
            let Some(icon) = icon else { continue };
            let kind = type_for_icon(icon);
            let emoji = kind.map(emoji_for_type).unwrap_or(icon).to_owned();
            let (n, lower) = wire.count();
            add(&mut out, kind, emoji, n, lower, None);
        }
    }
    out.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.r_type.cmp(&b.r_type))
            .then_with(|| a.emoji.cmp(&b.emoji))
    });
    out
}

/// Merge snapshot reactors that have no local event ledger; never rewind tracked reactors.
pub fn merge_untracked_snapshot(
    summaries: &mut Vec<ReactionSummary>,
    incoming: &[ReactionSummary],
    tracked: &BTreeMap<String, ReactionVersion>,
    replace_missing: bool,
) {
    let users: std::collections::BTreeSet<_> = incoming
        .iter()
        .chain(summaries.iter().filter(|_| replace_missing))
        .flat_map(|s| s.reactor_counts.keys())
        .filter(|user| !tracked.contains_key(*user))
        .cloned()
        .collect();
    for user in users {
        apply_reaction_event(
            summaries,
            &ReactionEvent {
                sender_id: user.clone(),
                r_type: -1,
                ..Default::default()
            },
        );
        for summary in incoming {
            if let Some(n) = summary.reactor_counts.get(&user) {
                add(
                    summaries,
                    summary.r_type,
                    summary.emoji.clone(),
                    *n,
                    summary.count_is_lower_bound,
                    Some(&user),
                );
            }
        }
    }
}

pub fn legacy_emojis(summaries: &[ReactionSummary]) -> Vec<String> {
    summaries
        .iter()
        .filter(|s| s.count > 0)
        .map(|s| s.emoji.clone())
        .collect()
}

/// Parse the shipped `chat.reaction` envelope or an array of those envelopes.
/// Socket queue dispatch must identify the reaction channel before invoking this parser.
pub fn parse_reaction_events(value: &Value) -> Vec<ReactionEvent> {
    if let Some(items) = value.as_array() {
        return items.iter().flat_map(parse_reaction_events).collect();
    }
    let wire = ReactionEventWireData::from_value(value);
    if wire.reacts.is_some() || wire.react_groups.is_some() {
        let mut events: Vec<_> = wire
            .reacts
            .iter()
            .flatten()
            .flat_map(parse_reaction_events)
            .collect();
        for item in wire.react_groups.iter().flatten() {
            let mut item = item.clone();
            let group_id = id(item.get("idTo"));
            if group_id.is_empty() {
                continue;
            }
            item["idTo"] = Value::String(format!("g{group_id}"));
            events.extend(parse_reaction_events(&item));
        }
        return events;
    }
    if let Some(items) = &wire.reactions {
        return items.iter().flat_map(parse_reaction_events).collect();
    }
    if let Some(items) = &wire.messages {
        return items
            .iter()
            .filter(|item| item.get("msgType").and_then(Value::as_str) == Some("chat.reaction"))
            .flat_map(parse_reaction_events)
            .collect();
    }
    let Some(content) = wire.reaction_content() else {
        return Vec::new();
    };
    let Some(r_type) = content.r_type() else {
        return Vec::new();
    };
    if r_type < -1 {
        return Vec::new();
    }
    let sender_id = wire.sender_id();
    if sender_id.is_empty() {
        return Vec::new();
    }
    let mut thread_id = wire.thread_id();
    if thread_id == "0" {
        thread_id = sender_id.clone();
    }
    if thread_id.is_empty() {
        return Vec::new();
    }
    content
        .targets()
        .iter()
        .filter_map(|target| {
            let target = ReactionTargetWireData::from_value(target);
            let mut message_id = target.message_id();
            let client_message_id = target.client_message_id();
            if message_id == "0" {
                message_id.clear();
            }
            if message_id.is_empty() && (client_message_id.is_empty() || client_message_id == "0") {
                return None;
            }
            Some(ReactionEvent {
                message_id,
                client_message_id,
                thread_id: thread_id.clone(),
                sender_id: sender_id.clone(),
                event_id: id(wire.msg_id.as_ref()),
                client_event_id: id(wire.cli_msg_id.as_ref()),
                r_type,
                emoji: content
                    .r_icon()
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| icon_to_emoji(s).to_string())
                    .unwrap_or_else(|| emoji_for_type(r_type).to_string()),
                historical: false,
                ts: wire.timestamp(),
            })
        })
        .collect()
}

/// Apply one deduplicated incremental event. The caller owns replay/order handling.
pub fn apply_reaction_event(summaries: &mut Vec<ReactionSummary>, event: &ReactionEvent) {
    if event.r_type == -1 {
        for summary in summaries.iter_mut() {
            if let Some(n) = summary.reactor_counts.remove(&event.sender_id) {
                summary.count = summary.count.saturating_sub(n);
                summary.reactor_ids.retain(|id| id != &event.sender_id);
            }
        }
        summaries.retain(|s| s.count > 0);
    } else {
        add(
            summaries,
            Some(event.r_type),
            event.emoji.clone(),
            1,
            false,
            Some(&event.sender_id),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn heart_count_is_not_unique_user_count() {
        let r =
            parse_reactions(&json!({"reacts":{"reactions":{"other":{"5":2},"0":{"5":1,"3":1}}}}));
        assert_eq!(r[0].emoji, "❤️");
        assert_eq!(r[0].count, 3);
        assert_eq!(r[0].reactor_ids.len(), 2);
        assert_eq!(r[1].count, 1);
    }
    #[test]
    fn snapshot_and_cache_defaults() {
        let r: ReactionSummary = serde_json::from_str("{}").unwrap();
        assert_eq!(r, ReactionSummary::default());
        let r = parse_reactions(
            &json!({"reactions":{"user":{"5":"2","3":"9k+","-1":1,"20":0,"2":-1}}}),
        );
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].count, 9001);
        assert!(r[0].count_is_lower_bound);
        assert_eq!(
            serde_json::from_str::<Vec<ReactionSummary>>(&serde_json::to_string(&r).unwrap())
                .unwrap(),
            r
        );
    }
    #[test]
    fn unknown_is_not_heart() {
        let r = parse_reactions(&json!({"reactions":{"user":{"999":2}}}));
        assert_eq!(r[0].r_type, Some(999));
        assert_ne!(r[0].emoji, "❤️");
    }
    #[test]
    fn legacy_shapes() {
        assert_eq!(parse_reactions(&json!({"reactions":["❤️","👍"]})).len(), 2);
        assert_eq!(
            parse_reactions(&json!({"reacts":{"reactions":[{"rIcon":"/-heart","count":2}]}}))[0]
                .count,
            2
        );
        assert!(parse_reactions(&json!({"reacts":null})).is_empty());
    }
    #[test]
    fn event_ids_remain_lossless_and_multitarget() {
        let r = parse_reaction_events(
            &json!({"uidFrom":"other","idTo":"0","msgId":"18446744073709551614","ts":"100", "content":r#"{"rType":5,"rIcon":"/-heart","rMsg":[{"gMsgID":18446744073709551613,"cMsgID":100},{"gMsgID":"0","cMsgID":"101"}]}"#}),
        );
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].message_id, "18446744073709551613");
        assert_eq!(r[0].thread_id, "other");
        assert_eq!(r[1].client_message_id, "101");
        assert!(r[1].message_id.is_empty());
    }
    #[test]
    fn removal_resets_all_types_for_sender_only() {
        let mut r = parse_reactions(&json!({"reactions":{"a":{"5":2,"3":1},"b":{"5":1}}}));
        apply_reaction_event(
            &mut r,
            &ReactionEvent {
                sender_id: "a".into(),
                r_type: -1,
                ..Default::default()
            },
        );
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].count, 1);
        assert_eq!(r[0].reactor_ids, vec!["b"]);
        apply_reaction_event(
            &mut r,
            &ReactionEvent {
                sender_id: "b".into(),
                r_type: 5,
                emoji: "❤️".into(),
                ..Default::default()
            },
        );
        assert_eq!(r[0].count, 2);
    }
    #[test]
    fn polling_single_message_normalization() {
        let r = parse_reaction_events(
            &json!({"messages":[{"msgType":"chat.reaction","msgKey":"peer","uidFrom":"peer","cliMsgId":"12","content":{"params":"{\"rType\":5,\"rMsg\":[{\"gMsgID\":\"42\"}]}"}}]}),
        );
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].thread_id, "peer");
        assert_eq!(r[0].client_event_id, "12");
        let r = parse_reaction_events(
            &json!({"reactions":[{"uidFrom":"peer","idTo":"0","content":{"rType":5,"rMsg":[{"gMsgID":"42"}]}}]}),
        );
        assert_eq!(r.len(), 1);
    }
    #[test]
    fn control_reaction_arrays_preserve_group_thread() {
        let r = parse_reaction_events(
            &json!({"reacts":[{"uidFrom":"peer","idTo":"0","content":{"rType":5,"rMsg":[{"gMsgID":"1"}]}}],"reactGroups":[{"uidFrom":"peer","idTo":"99","content":{"rType":3,"rMsg":[{"gMsgID":"2"}]}}]}),
        );
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].thread_id, "peer");
        assert_eq!(r[1].thread_id, "g99");
    }
    #[test]
    fn malformed_events_do_not_create_reactions() {
        for v in [
            json!({"content":"bad"}),
            json!({"uidFrom":"a","content":{"rType":5,"rMsg":[{}]}}),
            json!({"uidFrom":"a","idTo":"b","content":{"rType":-2,"rMsg":[{"gMsgID":"1"}]}}),
        ] {
            assert!(parse_reaction_events(&v).is_empty());
        }
    }
    #[test]
    fn simple_hash_matches_zalo_reaction_spec() {
        assert_eq!(simple_hash("👏"), 1772466);
        assert_eq!(simple_hash("🎉"), 1773261);
        assert_eq!(simple_hash("🔥"), 1772680);
        assert_eq!(simple_hash("👍"), 1772464);
        assert_eq!(simple_hash("hello"), 99162322);
        assert_eq!(simple_hash("xin chào Việt Nam"), 1766506019);
        assert_eq!(simple_hash("❤️"), 377643);
    }

    #[test]
    fn reaction_code_and_type_maps_canonical_and_custom() {
        assert_eq!(
            reaction_code_and_type("👍"),
            (std::borrow::Cow::Borrowed("/-strong"), 3)
        );
        assert_eq!(
            reaction_code_and_type("❤️"),
            (std::borrow::Cow::Borrowed("/-heart"), 5)
        );
        assert_eq!(
            reaction_code_and_type("👏"),
            (std::borrow::Cow::Borrowed(":handclap"), 46)
        );
        assert_eq!(
            reaction_code_and_type(":handclap"),
            (std::borrow::Cow::Borrowed(":handclap"), 46)
        );
        assert_eq!(
            reaction_code_and_type("🎉"),
            (std::borrow::Cow::Borrowed("🎉"), 101)
        );
        let fire_hash = simple_hash("🔥");
        assert_eq!(
            reaction_code_and_type("🔥"),
            (std::borrow::Cow::Owned("🔥".into()), fire_hash)
        );
    }

    #[test]
    fn custom_and_extended_ricon_preserved_in_reaction_events() {
        let r = parse_reaction_events(&json!({
            "uidFrom": "peer",
            "idTo": "0",
            "content": {
                "rType": 1772680,
                "rIcon": "🔥",
                "rMsg": [{"gMsgID": "42", "cMsgID": 100}]
            }
        }));
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].emoji, "🔥");
        assert_eq!(r[0].r_type, 1772680);

        let r_clap = parse_reaction_events(&json!({
            "uidFrom": "peer",
            "idTo": "0",
            "content": {
                "rType": 46,
                "rIcon": ":handclap",
                "rMsg": [{"gMsgID": "42", "cMsgID": 100}]
            }
        }));
        assert_eq!(r_clap.len(), 1);
        assert_eq!(r_clap[0].emoji, "👏");
        assert_eq!(r_clap[0].r_type, 46);
    }
}

/// Persist source and capture watermark independently of the transient network-fresh bit.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct ReactionSnapshotProvenance {
    pub source: String,
    pub captured_at_ms: i64,
}
impl ReactionSnapshotProvenance {
    /// A stale baseline cannot erase a newer live event or a clear tombstone.
    pub fn accepts_snapshot(
        &self,
        incoming: &Self,
        versions: &BTreeMap<String, ReactionVersion>,
        network_fresh: bool,
    ) -> bool {
        if !network_fresh && !versions.is_empty() {
            return false;
        }
        if self.captured_at_ms > 0 && incoming.captured_at_ms <= self.captured_at_ms {
            return false;
        }
        let latest_live = versions
            .values()
            .flat_map(|v| v.events.iter().chain(&v.covered_events))
            .filter(|e| !e.historical)
            .map(|e| crate::clock::normalize_ts_ms(e.ts))
            .max()
            .unwrap_or(0);
        latest_live == 0 || incoming.captured_at_ms >= latest_live
    }
}

/// Persist unresolved events to avoid reporting complete reaction coverage after a missed lookup.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReactionCoverage {
    pub unresolved: Vec<ReactionEvent>,
}
impl ReactionCoverage {
    pub fn record_unresolved(&mut self, event: &ReactionEvent) {
        if !self.unresolved.iter().any(|old| {
            old.thread_id == event.thread_id
                && old.message_id == event.message_id
                && old.client_message_id == event.client_message_id
                && old.sender_id == event.sender_id
                && same_event(old, event)
        }) {
            self.unresolved.push(event.clone());
        }
    }
    pub fn is_complete(&self) -> bool {
        self.unresolved.is_empty()
    }
}

/// Convert Zalo reaction icon code to visual emoji.
pub fn icon_to_emoji(icon: &str) -> &str {
    match icon.trim() {
        "/-strong" => "👍",
        "/-heart" => "❤️",
        ":>" => "😆",
        ":o" => "😮",
        ":-((" | ":-(( " => "😢",
        ":-h" => "😠",
        ":handclap" => "👏",
        "/-rose" => "🌹",
        "/-break" => "💔",
        "/-weak" => "👎",
        "/-ok" => "👌",
        "/-v" => "✌️",
        "/-thanks" => "🙏",
        "/-shit" => "💩",
        "" => "❤️",
        other => other,
    }
}

/// Deterministic positive 32-bit hash identical to `simpleHash` in `zalo-reaction`.
pub fn simple_hash(s: &str) -> i32 {
    let mut hash: i32 = 0;
    for c in s.encode_utf16() {
        hash = (hash.wrapping_shl(5))
            .wrapping_sub(hash)
            .wrapping_add(c as i32);
    }
    (hash as i64).abs() as i32
}

/// Map a visual reaction emoji or Zalo reaction code to `(rIcon, rType)`.
pub fn reaction_code_and_type(emoji_or_code: &str) -> (std::borrow::Cow<'static, str>, i32) {
    let trimmed = emoji_or_code.trim();
    if let Some(t) = type_for_icon(trimmed) {
        let icon = match t {
            3 => "/-strong",
            5 => "/-heart",
            0 => ":>",
            32 => ":o",
            2 => ":-((",
            20 => ":-h",
            46 => ":handclap",
            120 => "/-rose",
            65 => "/-break",
            4 => "/-weak",
            68 => "/-ok",
            69 => "/-v",
            70 => "/-thanks",
            66 => "/-shit",
            101 => "🎉",
            _ => "/-heart",
        };
        return (std::borrow::Cow::Borrowed(icon), t);
    }
    (
        std::borrow::Cow::Owned(trimmed.to_string()),
        simple_hash(trimmed),
    )
}

fn resolve_target_msg_type(msg: &super::threads::ChatMessage) -> i64 {
    if let Some(t) = crate::protocol::wire::val_i64(msg.wire_ref.original_type.as_ref()) {
        if t > 0 {
            return t;
        }
    }
    if msg.sticker_id.is_some() {
        4
    } else if msg.link_url.is_some() {
        1805
    } else {
        1
    }
}

/// Send a message reaction (`sendReactionMessage`, cmd `12410` / `12411`).
///
/// # Errors
/// Returns an error if unauthenticated, encryption fails, or the HTTP call fails.
pub async fn send_reaction(
    s: &crate::protocol::session::Session,
    to_id: &str,
    is_group: bool,
    msg: &super::threads::ChatMessage,
    reaction: &str,
) -> anyhow::Result<crate::protocol::wire::ActionAck> {
    crate::protocol::wire::require_login(s)?;
    let now = crate::clock::now_ms();
    let (r_icon, r_type) = reaction_code_and_type(reaction);
    let is_local_id = msg.id.is_empty()
        || msg.id.starts_with("out_")
        || msg.id.starts_with("local_")
        || msg.id.starts_with('.');
    let g_msg_id_val = if is_local_id {
        serde_json::json!("")
    } else {
        serde_json::json!(msg.id)
    };
    let c_msg_id = if !msg.client_message_id.is_empty() {
        msg.client_message_id
            .parse::<u64>()
            .unwrap_or(if msg.ts > 0 { msg.ts as u64 } else { now })
    } else if msg.ts > 0 {
        msg.ts as u64
    } else {
        msg.id.parse::<u64>().unwrap_or(now)
    };

    let inner_msg = serde_json::json!({
        "rMsg": [{
            "gMsgID": g_msg_id_val,
            "cMsgID": c_msg_id,
            "msgType": resolve_target_msg_type(msg)
        }],
        "rIcon": r_icon.as_ref(),
        "rType": r_type,
        "source": 6
    })
    .to_string();

    let react_list = serde_json::json!([{
        "message": inner_msg,
        "clientId": now
    }]);

    let (url, body) = if is_group {
        let clean_grid = crate::protocol::wire::strip_group_prefix(to_id);
        (
            format!(
                "{}/api/group/reaction?{}",
                crate::endpoints::REACTION_API,
                s.common_params()
            ),
            crate::protocol::wire::obj(&[
                ("react_list", react_list),
                ("grid", serde_json::json!(clean_grid)),
                ("imei", serde_json::json!(s.imei)),
            ]),
        )
    } else {
        (
            format!(
                "{}/api/message/reaction?{}",
                crate::endpoints::REACTION_API,
                s.common_params()
            ),
            crate::protocol::wire::obj(&[
                ("react_list", react_list),
                ("toid", serde_json::json!(to_id)),
            ]),
        )
    };

    let params = s.encoded_params(&body)?;
    let resp = s.post_form(&url, format!("params={params}")).await?;
    let dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
    let ack = crate::protocol::wire::ActionAck::from_wire(&dec);
    if !ack.is_ok() {
        anyhow::bail!(
            "Zalo reaction rejected by server (code {:?}): {}",
            ack.err_code(),
            ack.error_message.as_deref().unwrap_or("unknown error")
        );
    }
    Ok(ack)
}
