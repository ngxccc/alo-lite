use std::borrow::Cow;
use std::error::Error;
use std::fs;
use std::path::Path;

use alolite_core::common::logger::{FindingLevel, LogEntry, ScanFinding};
use alolite_core::core::ipc::{
    CoreBootstrapSnapshot, CoreEventDto, DevCryptoInfo, DevDiagnosticsSnapshot, ImportSummaryDto,
    PinnedPackMeta,
};
use alolite_core::core::settings::AppSettings;
use alolite_core::protocol::messaging::links::LinkMetadata;
use alolite_core::protocol::messaging::model::{
    DeliveryState, MessageKind, UnreadPolicy, WireMessageRef,
};
use alolite_core::protocol::messaging::reactions::{
    ReactionEvent, ReactionSnapshotProvenance, ReactionSummary, ReactionVersion,
};
use alolite_core::protocol::messaging::stickers::{StickerItem, StickerPackInfo};
use alolite_core::protocol::messaging::threads::entity::{
    SidebarMember, SidebarPresentation, Thread,
};
use alolite_core::protocol::messaging::threads::message::ChatMessage;
use alolite_core::protocol::messaging::unread::mute::MuteControlEvent;
use alolite_core::protocol::messaging::unread::state::UnreadState;
use alolite_core::protocol::social::friends::{FriendshipStatus, PhoneLookupResult};
use alolite_core::services::account_manager::AccountProfile;
use alolite_core::services::updater::UpdateCheckResult;

use specta::datatype::{DataType, Fields, NamedReferenceType, Primitive, Reference};
use specta::{Format, Types};
use specta_typescript::Typescript;

struct IdentityFormat;

impl Format for IdentityFormat {
    fn map_types<'a>(
        &'a self,
        types: &Types,
    ) -> Result<Cow<'a, Types>, Box<dyn Error + Send + Sync + 'static>> {
        Ok(Cow::Owned(types.clone()))
    }

    fn map_type<'a>(
        &'a self,
        _types: &Types,
        dt: &DataType,
    ) -> Result<Cow<'a, DataType>, Box<dyn Error + Send + Sync + 'static>> {
        Ok(Cow::Owned(dt.clone()))
    }
}

fn remap_integers_to_number(dt: &mut DataType) {
    match dt {
        DataType::Primitive(p) => match p {
            Primitive::usize
            | Primitive::isize
            | Primitive::i64
            | Primitive::u64
            | Primitive::u128
            | Primitive::i128 => {
                *p = Primitive::f64;
            }
            _ => {}
        },
        DataType::List(l) => remap_integers_to_number(&mut l.ty),
        DataType::Map(m) => {
            remap_integers_to_number(m.key_ty_mut());
            remap_integers_to_number(m.value_ty_mut());
        }
        DataType::Struct(s) => match &mut s.fields {
            Fields::Unit => {}
            Fields::Unnamed(unnamed) => {
                for f in &mut unnamed.fields {
                    if let Some(ty) = &mut f.ty {
                        remap_integers_to_number(ty);
                    }
                }
            }
            Fields::Named(named) => {
                for (_, f) in &mut named.fields {
                    if let Some(ty) = &mut f.ty {
                        remap_integers_to_number(ty);
                    }
                }
            }
        },
        DataType::Enum(e) => {
            for (_, v) in &mut e.variants {
                match &mut v.fields {
                    Fields::Unit => {}
                    Fields::Unnamed(unnamed) => {
                        for f in &mut unnamed.fields {
                            if let Some(ty) = &mut f.ty {
                                remap_integers_to_number(ty);
                            }
                        }
                    }
                    Fields::Named(named) => {
                        for (_, f) in &mut named.fields {
                            if let Some(ty) = &mut f.ty {
                                remap_integers_to_number(ty);
                            }
                        }
                    }
                }
            }
        }
        DataType::Tuple(t) => {
            for elem in &mut t.elements {
                remap_integers_to_number(elem);
            }
        }
        DataType::Nullable(inner) => remap_integers_to_number(inner),
        DataType::Intersection(inners) => {
            for inner in inners {
                remap_integers_to_number(inner);
            }
        }
        DataType::Reference(Reference::Named(r)) => {
            if let NamedReferenceType::Inline { dt, .. } = &mut r.inner {
                remap_integers_to_number(dt);
            }
        }
        _ => {}
    }
}

pub fn generate_ipc_typescript() -> String {
    let types = Types::default()
        .register::<CoreEventDto>()
        .register::<CoreBootstrapSnapshot>()
        .register::<DevDiagnosticsSnapshot>()
        .register::<ImportSummaryDto>()
        .register::<PinnedPackMeta>()
        .register::<DevCryptoInfo>()
        .register::<AppSettings>()
        .register::<AccountProfile>()
        .register::<LogEntry>()
        .register::<ScanFinding>()
        .register::<FindingLevel>()
        .register::<UpdateCheckResult>()
        .register::<FriendshipStatus>()
        .register::<PhoneLookupResult>()
        .register::<MuteControlEvent>()
        .register::<UnreadState>()
        .register::<StickerItem>()
        .register::<StickerPackInfo>()
        .register::<LinkMetadata>()
        .register::<ReactionSummary>()
        .register::<ReactionEvent>()
        .register::<ReactionVersion>()
        .register::<ReactionSnapshotProvenance>()
        .register::<MessageKind>()
        .register::<WireMessageRef>()
        .register::<UnreadPolicy>()
        .register::<DeliveryState>()
        .register::<SidebarMember>()
        .register::<SidebarPresentation>()
        .register::<Thread>()
        .register::<ChatMessage>();

    let types = specta_serde::Format
        .map_types(&types)
        .expect("serde formatting failed");

    let types = types.into_owned().map(|mut ndt| {
        if let Some(dt) = &mut ndt.ty {
            remap_integers_to_number(dt);
        }
        ndt
    });

    let header = concat!(
        "// Auto-generated by Specta (AloLite Code-as-SSOT Architecture).\n",
        "// DO NOT EDIT MANUALLY: changes will be overwritten by `cargo test -p alolite-core`.\n",
        "/* eslint-disable */\n",
        "/* tslint:disable */\n\n",
    );

    let ts = Typescript::default().header(header);
    ts.export(&types, IdentityFormat)
        .expect("Failed to export types")
}

#[test]
fn test_ipc_types_sync_with_frontend() {
    let output = generate_ipc_typescript();
    let dest_path = Path::new("../../apps/desktop/src/shared/types/ipc.generated.ts");

    if dest_path.exists() {
        let current_content = fs::read_to_string(dest_path).unwrap_or_default();
        if current_content != output {
            // Write updated content so developers automatically get the refreshed file
            let _ = fs::write(dest_path, &output);
            assert_eq!(
                current_content, output,
                "Generated IPC types were out of sync and have been updated. Please re-run test to confirm."
            );
        }
    } else {
        let _ = fs::write(dest_path, &output);
    }
}
