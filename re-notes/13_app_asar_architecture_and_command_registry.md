# 13 — `app.asar` Master Architecture, Cross-Platform Bundle Audit & Complete Command Registry

Date: 2026-11-08  
Status: Ground-Truth Verified (MCP REA + Static Archive Audit)  
Evidence tags: **[O]** Observed in official `Zalo 26.11.10` distributions (`macOS DMG`, `Windows EXE`, `zalo-for-linux`), **[V]** Cross-verified against live sessions and `alolite` protocol modules.

> **Clean-Room Compliance Notice (`docs/standards/compliance-and-ip.md`):** This document records only archive manifests, cryptographic digests, IPC/worker boundaries, WASM export tables, WebSocket frame IDs, and HTTP `(commandId, domain, path)` routing tables. No verbatim proprietary JavaScript or decompiled C/C++ source code is included.

---

## 1. Cross-Platform Bundle Provenance & Forensic Comparison [O, V]

All three desktop distributions inspected during this investigation originate from release **`26.11.10`** (`commit: 9480267`, `buildDate: 2026-11-04T09:45:09.128Z`, Electron `22.3.x` / Node ABI `110`):

| Distribution | Carrier Layout | Core JS & WASM Payload | Native Addon Profile | Forensic Delta Notes |
|:---|:---|:---|:---|:---|
| **macOS Universal** (`ZaloSetup-universal-26.11.10.dmg`) | `Zalo.app/Contents/Resources/app.asar` (`192,832,202` B, `sha256: 9217d0e4...`) + `app.asar.unpacked/` | Unmodified upstream (`175` ASAR entries; `compact-app-pc` = `21,676,308` B) | `darwin_arm64` & `darwin_x64` Mach-O `.node` / `.dylib` (`52` files post-modified by Apple `codesign`); `db-cross-v4-native.node` retains DWARF/C++ `LC_SYMTAB` symbols | Canonical upstream reference (`ev_f423a1d4...`, `ev_c8328ec6...`). |
| **Windows** (`ZaloSetup-26.11.10.exe` / `Zalo-Win-26.11.10.7z`) | Outer NSIS archive holds only `ZaloCap.dll`, `ZaviMeet.pdb`, and `plugins/capture/ocr/zmodels.zmdl`; full Electron app is nested inside **`$PLUGINSDIR/app-32.7z`** | Identical JS & WASM release (`26.11.10`) inside `app-32.7z:resources/app.asar` | 32-bit `PE32 Intel i386` `.node` addons (`win32_ia32`), symbol-stripped | Extracting only the outer installer misses `app.asar` unless `app-32.7z` is unpacked. |
| **Linux Wrapper** (`zalo-for-linux/app/`) | Extracted `app/` directory derived from macOS `26.11.10` DMG + `main.js` / `plugins/` / `scripts/patches/` | Patched in-place **without** renaming webpack contenthashes (`compact-app-pc` = `21,677,401` B, `+1,093` B starting at byte `3,283,500`) | Rebuilt Linux x64 N-API addons (`db-cross-v4`, `file-utilities`, `zimage`, `mp4thumb`, `zlang`) + prebuilt `node_sqlite3.node` | Applies 19 build-time JS patches (notably `patch-disk-space.js`, `patch-db-cross-v4.js` `LINUX -> 23`, and `patch-network-and-sync-fix.js` disabling `gwig` probe & forcing Sync V2 off). |

---

## 2. `app.asar` Internal Bundle & Worker Map [O]

### 2.1 Entrypoints, Preloads, Workers & Lazy Bundles

| Path inside `app.asar` | Size | Process / Thread | Responsibility |
|:---|---:|:---|:---|
| `package.json` | `3.9 KB` | Metadata | `version: "26.11.10"`, `main: "bootstrap.js"`; key runtime packages: `@oneidentity/zstd-js 1.0.3`, `@zsecure/zpc 0.3.0`, `lz4`, `spark-md5`, `uuid` |
| `bootstrap.js` → `main-dist/main.js` | `8.2 MB` | Electron Main | App lifecycle, window management, auto-updater, DNS/service-map pool, `iqBP` device identity (`type=23/24/25`, `ver=695`), `$z*` IPC handlers |
| `main-dist/utility-process-sqlite.js` | `1.1 MB` | UtilityProcess | Hosts `node_sqlite3.node` (SQLCipher v4 local database I/O, `--backupdata` worker) |
| `main-dist/utility-process-media.js` | `1.1 MB` | UtilityProcess | Local media thumbnailing, `libvips` / `libjxl` / `mp4thumb` offloading |
| `pc-dist/preload.js`, `main-dist/preload-shared-worker.js`, `main-dist/preload-sqlite.js` | `~6 KB` | Preload | Injects `$z*` globals (`contextBridge` / renderer-to-main shims) |
| `pc-dist/compact-app-pc.cfeba43a9a8e4e40dda3.js` | `20.7 MB` | Renderer | Primary UI shell, Redux/state containers, media viewers (`jxl_to_jpg` fallback) |
| `pc-dist/lazy/default-login-main-startup-shared-worker-znotification.4bb5e1e2c2e9bab4f7b3.js` | `17.2 MB` | Shared chunk | Master HTTP `commandId` registry, WebSocket `cmd` framing & router, `ParamCipher` (`encodeAES`), Signal `SessionCipher`/`GroupCipher` orchestrator |
| `pc-dist/lazy/main-startup.89bb530b91927fd5dd28.js` | `26.3 MB` | Renderer | Post-login workspace bootstrap, `SyncMessageController` (Sync V1 + RSA-2048), `SyncController` (Sync V2), `BackupMsg` controller |
| `pc-dist/lazy/login-startup.65eb8d358ddd85ba3f21.js` | `442 KB` | Renderer | Pre-login `getServerInfo`, QR1/QR2 `reqqr` state machine, `Le()` `logged_uids`, Trusted Device linking trigger |
| `pc-dist/shared-worker.cfeba43a9a8e4e40dda3.js` | `424 KB` | SharedWorker | `sync2-proto` WASM wrapper, `backup-proto` WASM wrapper, `ZstdStream` + `proto.Sync2.*` serializer, Sync V1 `ChatContent` SQLite reader |
| `pc-dist/sync-v2-sub-worker.cfeba43a9a8e4e40dda3.js` | `21.4 MB` | DedicatedWorker | Sync V2 chunk pipeline, `E2eeQueueOfflineController`, Signal decrypt workers |
| `pc-dist/trust-worker.cfeba43a9a8e4e40dda3.js` | `190 KB` | DedicatedWorker | Dedicated worker hosting `trusted_device_protocol_bg.wasm` |
| `pc-dist/zd-worker.cfeba43a9a8e4e40dda3.js` | `7.8 MB` | DedicatedWorker | Diagnostics, logging (`zsymb`), and QoS metrics |
| `pc-dist/search-worker.*.js`, `gpu-heavy-worker.*.js`, `preview-thumb-worker.*.js` | `40–191 KB` | DedicatedWorkers | Full-text search indexing, canvas/blur/image processing |

### 2.2 Preload / IPC Bridge Surface (`$z*` Globals) [O]
Renderer and worker contexts communicate with Electron Main and native Node modules through 42 `$z*` bridge namespaces:

`$zapp`, `$zbackup`, `$zcall`, `$zcloud`, `$zcommon`, `$zconfig`, `$zdb`, `$zdownload`, `$zelectron`, `$zelectronNative`, `$zent_chunk_fetch_profile`, `$zenv`, `$zFeatures`, `$zFileManager`, `$zInAppPayment`, `$zlogger`, `$zlogin`, `$zMetric`, `$zMigrateDrive`, `$zMsgChannel`, `$zmulti`, `$znetwork`, `$znode` (`os`, `path`, `crypto`, `nativelibs`), `$znoti`, `$zpc`, `$zperf`, `$zpinVault`, `$zpreload`, `$zprotocol`, `$zresource`, `$zsafeStorage` (OS keychain encryption for local secrets), `$zscreencap`, `$zscript`, `$zsessionManager`, `$zsharedWorker`, `$zsqlite`, `$zsqlitebw`, `$zsub`, `$zTFModels` (local ONNX/TFJS inference), `$zupdater`, `$zuri`, `$zwindow`.

---

## 3. Complete WebAssembly (`pc-dist/libs/*.wasm`) Export Inventory [O]

1. **`trusted_device_protocol_bg.97af2bf51fb002cefc5a888e932bcab2.wasm`** (`346,340` B, `sha256: 716f0a06...` — full crypto spec in [note 11](11_trusted_device_protocol.md)):
   - `zprotoEd25519GenerateKeyPair`, `zprotoTrustGenerateLinkingSecretKey`, `zprotoTrustGenerateLinkingQr`, `zprotoTrustGetRelinkId`, `zprotoTrustCreateCompanionLinkingProof`, `zprotoTrustVerifyDeviceListFromBytes`, `zprotoTrustVerifyDeviceListFromStruct`, `zprotoTrustVerifyTrustedIdentity`, `zprotoTrustCheckExpiryTime`.
2. **`libzproto_wasm_bg.5ea0fb2c9af4a45bc7882b71307b6f83.wasm`** (`287,049` B, `sha256: 398a2016...` — `39` exports; combines Trusted Device + Ed25519/X25519 + **Sync V2**; see [note 12 §3.3.2](12_message_sync_and_e2ee_protocol.md)):
   - Logging: `zprotoLoggerInitialize`
   - Primitives: `zprotoEd25519GenerateKeyPair`, `zprotoEd25519DerivePublicKey`, `zprotoEd25519CalculateAgreement`, `zprotoEd25519CalculateSignature`, `zprotoEd25519VerifySignature` (+ `zprotoed25519keypair_*` accessors)
   - Sync V2: `zprotoSync2GetSessionId`, `zprotoSync2CreateMetadataCipher`, `zprotoSync2DecryptMetadata`, `zprotoSync2EncryptMessage`, `zprotoSync2DecryptMessage` (+ `zprotosyncmetadatacreated_*`, `zprotosyncmetadatadecrypted_*`, `zprotosyncmessageencrypted_*`, `zprotosyncmessagedecrypted_*` struct accessors)
   - Trusted Device (shared crate): `zprotoTrustGenerateLinkingSecretKey`, `zprotoTrustGenerateLinkingQr`, `zprotoTrustGetRelinkId`, `zprotoTrustCreateCompanionLinkingProof`, `zprotoTrustVerifyDeviceListFromBytes`, `zprotoTrustVerifyDeviceListFromStruct`, `zprotoTrustVerifyTrustedIdentity`, `zprotoTrustCheckExpiryTime`.
3. **`libzproto_backup_wasm_bg.042497d03f0f504b6b1444e25a7db0d9.wasm`** (`129,796` B, `sha256: 3a980db6...` — `10` exports; **PC-to-Mobile Backup**; see [note 12 §3.4](12_message_sync_and_e2ee_protocol.md)):
   - `zprotoLoggerInitialize`, `zprotobackupcryptosession_new`, `zprotobackupcryptosession_encryptMessage`, `zprotobackupcryptosession_decryptMessage`, `__wbg_zprotobackupcryptosession_free`.

---

## 4. Complete Non-E2EE WebSocket Command & Close-Code Registry [O]

*(For E2EE WebSocket `SignalCommands` `514..582`, see [note 12 §4.3](12_message_sync_and_e2ee_protocol.md).)*

### 4.1 WebSocket Frame `cmd` Constants

| `cmd` | Symbolic Name | Plane / Purpose |
|---:|:---|:---|
| `1` | `AUTHEN` | Initial WebSocket session authentication handshake |
| `2` | `PING` | Keep-connection heartbeat (`KEEP_CONNECTION = 1`) |
| `4` | `PING_ACTIVE` | Active user presence heartbeat (`KEEP_ACTIVE = 2`) |
| `501` | `PUSH_MSG_1_1` | Real-time incoming 1-1 plaintext/standard message push |
| `502` | `PUSH_STATUS_1_1` | Real-time 1-1 typing / delivered / seen status push |
| `504` | `CLEAR_UNREADS_1_1` | Cross-device unread badge clear for 1-1 threads |
| `521` | `PUSH_MSG_GROUP` | Real-time incoming group message push |
| `522` | `PUSH_STATUS_GROUP` | Real-time group delivered / seen status push |
| `524` | `CLEAR_UNREADS_GROUP` | Cross-device unread badge clear for group threads |
| `530` | `PUSH_LIST_INTERACTIVE_FILE` | Interactive file state push |
| `532` | `VERIFY_MSG` | Message integrity / queue verification |
| `533` | `HAS_MSG` | Queue notification prompting client to pull via `GET_MSG` (`570`) |
| `534` | `PUSH_MISS_MSG` | Missed standard messages replayed after reconnect |
| `536` / `537` | `SUBMIT_INTERACTIVE_FILE` / `PULL_LIST_INTERACTIVE_FILE` | Interactive file sync |
| `570` | `GET_MSG` | Batch pull of queued messages (`max_msg_get = 20`, `{queueId, ids, lastId, reqId}`) |
| `571` | `PRELOAD` | Initial conversation/message preload over WS |
| `575` / `576` | `GET_STATUS_MSG` / `GET_STATUS_MSG_GROUP` | Fetch delivery/read status for 1-1 / group |
| `577` / `578` | `GET_LAST_DELIVER_SEEN` / `GET_LAST_SEEN_CHAT_CONTAINER` | Fetch read-point cursors (`oneone`, `group`) |
| `590` / `591` / `592` | `SYNC_MESSAGE.{REQUEST, ACK_DELETE_SYNC_SESSION, REQUEST_MOBILE_WAKE_UP}` | Sync V2 control & wake-up ([note 12 §3.3](12_message_sync_and_e2ee_protocol.md)) |
| `600` | `CTRL_TESTING` | Control-plane diagnostic probe |
| `601` | `CTRL_PUSH_CTRL` | Real-time control event push (`syncmsgmb`, `transfer_msg`, `transfer_sync2`, `file_done`, `mute`, `pin`, `group`, `emoji_status`, `mark_unread`, `quick_message`, `prod_catalog`, `ban_media`, `my_settings`) |
| `602` | `CTRL_ACTION` | Client-originated control action |
| `603` | `CTRL_PULL_CTRL` | Pull queued offline control events |
| `604` | `CTRL_PULL_VOIP` | Pull VoIP signaling control events |
| `610` / `611` | `CTRL_PULL_REACT` / `CTRL_PULL_REACT_BIG_GR` | Pull reactions (standard / large group) |
| `612` / `613` / `614` | `CTRL_PUSH_REACT` / `CTRL_PUSH_CLEAR_REACT` / `CTRL_GET_LAST_SEEN_REACT` | Real-time reaction push, clear, and cursor sync |
| `620` / `621` / `623` | `CLOUD.{PUSH, NEED_VERIFY, DONE_MIGRATE}` | My Cloud / media drive push & migration events |
| `630..634` | `BACKUP_MSG.{INIT, CREATE_SESSION, GET_METADATA, SIGNAL_RESTORE, GET_CONFIGS}` | PC-to-Mobile backup session commands ([note 12 §3.4](12_message_sync_and_e2ee_protocol.md)) |
| `701` | `GET_IN_APP_PAYMENT_LINK` | In-app payment link resolution |
| `800` | `GET_MEDIA_UPLOAD_URL` | Multi-site CDN upload URL allocation |
| `3000` | `MAX_CONNECTION` | Concurrent socket cap notification |

### 4.2 Custom WebSocket Close & Error Codes
- Application close codes: `3000` (`NOT_AUTHENTICATED`), `3001` (`READ_MAX_CONNECTION`), `3002` (`NOT_FOUND_WORKER`), `3003` (`KICK_CONNECTION` — session superseded/logged out remotely).
- Internal socket error codes: `2021` (`RECONNECT_BY_PING_TIMEOUT`), `2022` (`DECOMPRESS_FAIL`), `5017..5020` (offline control/close/reconnect timeouts).

---

## 5. Master HTTP `commandId` Routing Registry (`app.asar`) [O]

### 5.1 Authentication, Session & Account Settings (`auth` / `wpa.chat.zalo.me`)

| `commandId` | Endpoint Path | `commandId` | Endpoint Path |
|---:|:---|---:|:---|
| `11133` | `/api/login/reqqr` | `11335` | `/api/update/password` |
| `11134` | `/api/login/getLoginInfo` | `11336` | `/api/login/loginByFacebook` |
| `11135` | `/api/login/logOut` | `11337` | `/api/login/getActiveCodeByZalo` |
| `11160` | `/api/login/chkUpdate` | `11338` | `/api/login/linkFacebook` |
| `11172` | `/api/feedback` | `11339` | `/api/login/fblogout` |
| `11176` | `/api/login/loginByPasswv2` | `11341` / `11343` | `/api/login/fbAuth`, `/api/login/getFbAuthInfo` |
| `11177` | `/api/enterprise/login/emailPassword` | `11346` | `/api/login/actlistv2` |
| `11179` | `/api/login/completeRemoveData` | `11347` | `/api/register/verifyExt` |
| `11322` | `/api/login/getSupportCountry` | `12018` / `12021` | `/api/register/signUpWithPwd`, `/api/login/loginByResetPwd` |
| `11323` | `/api/login/getServerInfo` | `12019` | `/api/update/checkHasPassword` |
| `11325` | `/api/login/getCaptcha` | `12033` / `12113` | `/api/setting/update`, `/api/setting/me` |
| `11327` / `11329` | `/api/login/popup`, `/api/login/setting` | `12066` | `/api/update/setting/2FA` |
| `11330..11333` | `/api/register/{verifyPhone,getActiveCodeBySms,getActiveCodeByVoice,validateActiveCode}` | `12110` / `12111` | `/api/setting/rating`, `/api/syncmsg/confirm` *(unused)* |
| `11334` | `/api/update/profile` | `12382` / `12600`..`12602` | `/api/login/{hasNewMsg,ssoSyncMp3,loginByFacebookWithSecure,migrateSS}` |

### 5.2 1-1 Messaging, Conversation State & Cloud/Group History (`chat`, `conversation`, `group_cloud_message`)

| `commandId` | Domain | Endpoint Path | `commandId` | Domain | Endpoint Path |
|---:|:---|:---|---:|:---|:---|
| `10048`..`10051` | `conversation` | `/api/hiddenconvers/{get-all,add-remove,update-pin,reset}` | `11779` | `chat` | `/api/message/delete` |
| `10053` | `conversation` | `/api/preloadconvers/get-last-msgs` | `12009` | `chat` | `/api/preloadconvers/get-last-seen` |
| `11144` | `chat` | `/api/message/typing` | `12011`..`12014` | `conversation` | `/api/conv/{getUnreadMark,addUnreadMark,removeUnreadMark}` |
| `11150` / `12118` | `chat` | `/api/message/{seen,seenv2}` | `12016` | `chat` | `/api/message/getStatusMsg` |
| `11151` | `chat` | `/api/message/sms` | `12022` / `12023` | `conversation` | `/api/conv/autodelete/{getConvers,updateConvers}` |
| `11153` / `12117` | `chat` | `/api/message/{delivered,deliveredv2}` | `12076` / `12077` | `conversation` | `/api/archivedchat/{list,update}` |
| `11154` | `chat` | `/api/message/sticker` | `12114` | `chat` | `/api/message/fetchImportant` |
| `11233` | `conversation` | `/api/suggestconvers/list` | `12115` | `chat` | `/api/message/multi_undo` |
| `11281` | `chat` | `/api/message/undo` | `12120`..`12122` | `conversation` | `/api/pinconvers/{list,update,updatev2}` |
| `11286` | `chat` | `/api/message/link` | `12160` / `12161` | `label` | `/api/convlabel/{update,get}` |
| `11770` | `chat` | `/keepalive` | `12300` | `conversation` | `/api/conv/cm/grOfl` |
| `11771` | `chat` | `/api/message/quote` | `12503` / `12504` | `cloud_msg` | `/api/cm/{getpath,rgetv2}` |
| `11774` | `chat` | `/api/message/contact` | `12506` / `12508` / `12509` | `cloud_msg` | `/api/cm/{getoldv2,getrecentv2,mget}` |
| `11778` | `chat` | `/api/message/deleteconver` | `12040` | `cloud_msg` | `/api/cm/pc/dl` |

### 5.3 Groups, Communities, Boards, Polls & Reactions (`group`, `board`, `poll`, `reaction`)

| `commandId` | Endpoint Path | `commandId` | Endpoint Path |
|---:|:---|---:|:---|
| `10063`..`10065` | `/api/group/{change-owner,pending-mems/list,pending-mems/review}` | `12027` / `12146` | `/api/group/{getlg/v4,getlg/e2ee}` |
| `11161` / `11162` | `/api/group/{getlg/v3,getgi/v2}` | `12028` / `12604` | `/api/group/{getmg-v2,getmg}` |
| `11163` / `11165` | `/api/group/{sendmsg,sticker}` | `12116` / `12119` / `12124` | `/api/group/{multi_undomsg,deliveredv2,seenv2}` |
| `11166` / `11167` | `/api/group/{delivered,seen}` | `12170`..`12173` | `/api/group/inv-box/{list,mdel-inv,join,inv-info}` |
| `11185`..`11189` | `/api/group/{create,invite,kickout,updateinfo,leave}` | `12180`..`12182` | `/api/group/{upgrade/community,quota/community,quota/members-comm}` |
| `11192` / `11193` | `/api/group/{undomsg,sendlink}` | `12407` | `/api/group/analyse` |
| `11196`..`11198` | `/api/group/{typing,quote,mention}` | `12443` / `12444` | `/api/group/{create/v2,invite/v2}` |
| `11240` | `/api/group/history` | `12605`..`12607` | `/api/group/{invite/multi,kickout/multi,userctx/multiget}` |
| `11811`..`11816` | `/api/group/{blockedmems/list,blockedmems/add,blockedmems/remove,admins/add,admins/remove,setting/update}` | `12001`..`12010`, `12086`..`12088`, `12140`..`12145`, `12425`, `12429`..`12432` | `/api/board/*` (pins, group topics, reminders, 1-1 notes, personal todos `12414`..`12427`) |
| `11821`..`11825` | `/api/group/link/{detail,new,disable,ginfo,join}` | `12080`..`12085`, `12089` | `/api/poll/{create,detail,option/add,vote,share,end}` |
| `11827`..`11835` | `/api/group/{contact,topic/remove,disperse,deletemsg,deleteconver}` | `12410`, `12411`, `12423`, `12424`, `12448`..`12450` | `/api/{message,group}/{reaction,react_delivered,react_seen}`, `/api/react/gLastSn` |
| `12017` | `/api/group/getStatusMsg` | | |

### 5.4 Files, Media Upload, Sync/Transfer & Stickers (`file`, `zcloud`, `sticker`)

| `commandId` | Endpoint Path | `commandId` | Endpoint Path |
|---:|:---|---:|:---|
| `11145` / `11146` / `11148` / `11152` | `/api/message/{sharefile,sharefile_full,voicemp3,photo}` | `12058`..`12063` | `/api/{message,group}/{video/file,video/thumb,voice}/upload` |
| `11164` / `11169` / `11190` / `11191` / `11194` / `11195` | `/api/group/{sendpt,sendmp3,sharefile,sharefile_full,gif,upavatar}` | `12094` / `12400` | `/api/message/{renewlink,scanurl}` |
| `11199` / `11818` / `12440` / `12442` | `/api/{message,group}/{forward,mforward}` | `12651` / `12652` / `12671` / `12672` | `/api/{message,group}/{upload,zcloud-upload}/photo` |
| `11285` / `11292` / `11293` | `/api/message/{parselink,gif,upthumb}` | `13000` / `13001` | `/api/group/asyncfile/{upload,msg}` |
| `11772`..`11777` | `/api/message/{photo_original/upload,photo_original/send,photo_url,asyncfile/upload,asyncfile/msg}` | `11841`..`11843`, `12220`..`12222` | `/api/mediastore/{list,remove,preview}`, `/api/media/oneone/{list,preview,remove}` |
| `11819` / `11820` / `11826` / `11830` | `/api/profile/upavatar`, `/api/group/{photo_original/upload,photo_original/send,photo_url}` | `11155`, `11156`, `11279`, `11284`, `11294`..`11299`, `11740`..`11751`, `12015`, `12020`, `12190` | `/api/message/{sticker,effects,guggy,giphy}/*` |
| `12000`, `12003`, `12096`, `12412`, `12700`, `12888` | `/api/message/{pull_mobile_msg,delete_snapshot_mobile_msg,get_backupmsginfo,get_crossdb,cancel_pull_mobile_msg}`, `/api/transfer-sync-v2/request-sync` | `13538`, `15298`, `15306`..`15309` | `/api/backuppc2mb/upload`, `/api/transfer/{list,conf,card,create,update,delete}` |

### 5.5 Contacts, Profile, Alias, VoIP & Auxiliary Services (`profile`, `friend`, `alias`, `voice_call`)

| Category | `commandId` & Endpoint Paths |
|:---|:---|
| **Profile & Social (`profile`)** | `11137..11143` `/api/social/friend/{getfriends,verify,getprofiles/v2,getclosedfriends}`, `/api/social/profile/{me-v2,ping,deactive}`; `11280, 11288..11291` `{getmute,setmute,updatelang,friend/onlines,status}`; `11860..11865` `{avatar,friend/phonebook,oa/convertid,extra,mode}`; `12012` `getrank`; `12030` `group/members`; `12053/12054` `/api/gid/{encrypt,decrypt}`; `12072..12075` `{active-pcweb,get-bizacc,profile/update}`; `12090` `lastOnline`; `12130..12135` `{biz-register-trial,biz-off-remind,getminiprofiles,changefgtobg}`; `12150..12153` `{verification/status,ekyc/push-oa,biz-register-trial-v2,avatar-url}`; `12193` `/api/enterprise/profiles`; **`12390` `/api/znoise`**; `12428` `phonebook/v2`; `12451..12453` `{avatar-list,del-avatars,reuse-avatar}`; `12681/12682` `oa/{block,unblock}`; `15295` `/api/report/abuse-v2`; `31582` `reportabuse`. |
| **Friends (`friend`)** | `10004` `/api/friend/search/by-user-name`; `11420..11442` `/api/friend/{profile/get,closefr/update,block,remove,sendreq,accept,reject,blocklistv2,reqstatus,recommends/remove,unblock,recommendsv2/list,recommendsv2/remove,group/related,profile/multiget,undo,context/multiget}`; `12128` `mget-qr`; `12203/12204` `{requested/list,feed/block}`; `12230..12232` `{last-read-contact-tab,read-contact-tab,recommendsv2/read-requests}`. |
| **Aliases (`alias`)** | `12200..12202, 12205` `/api/alias/{list,update,remove,list/v2}`. |
| **1-1 & Group VoIP (`voice_call`)** | `11300..11311` `/api/voicecall/{requestcall,request,ringring,answer,answerack,cancel,endcall,holdreq,holdresp,unholdreq,unholdresp,logendcall}`; `12006` `conf`; `12043/12044` `group/{join,sharescreen}`; `12056` `requestaudio2video`; `12064` `requestsharescreen`; `12097/12098` `group/{ping,broadcast}`; `12433..12439, 12445..12447` `group/{requestcall,request,adduser,answer,cancel,endcall,ringring,state,finish,notify}`. |
| **QuickMsg, AutoReply, LAN & AI** | `12025, 12026, 12029, 12032` `/api/quickmessage/{list,update,create,delete}`; `12049..12051, 12067` `/api/autoreply/{list,create,update,delete}`; `12125, 12260..12263` `/api/lan/{samemacs,status,leave,submit,friends}`; `13396..13399` `/api/aifeatures/translate/v1/{setting,manual,auto}`. |
| **E2EE (`CHAT_E2EE` / `GROUP`)** | `16000..16012`, `16100..16113`, `16200..16204` — complete payload schemas in [note 12 §4.4](12_message_sync_and_e2ee_protocol.md). |
