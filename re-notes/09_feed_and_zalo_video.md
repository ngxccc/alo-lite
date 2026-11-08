# Social Feed (Nhật ký) & Zalo Video (Short Video) — Reverse Engineering Notes & Protocol Specification

* **Tài liệu**: Nghiên cứu & Dịch ngược Tính năng Nhật ký (Social Feed / Timeline) & Zalo Video.
* **Thời gian cập nhật**: Tháng 11/2026.
* **Mục tiêu dự án**: Đưa tính năng xem dòng thời gian (Feed), đăng bài (Post Feed) và xem trang cá nhân bạn bè lên client PC (`zalo-rust`).
* **Đối tượng phân tích (Artifact Targets)**:
  * Android APK: `com.zing.zalo.apk` (v26.10.02, `versionCode: 261002903`, targetSdk 36, 50.131 classes, 224.986 DEX strings).
  * Native Libraries: `lib/arm64-v8a/libnative_utils.so` (357 KB) và `lib/arm64-v8a/libznetwork.so`.
  * PC/Web Electron Bundle: `asar_extracted/pc-dist/compact-app-pc.b7bec208c4c8f73e09f8.js`.

---

## 1. Bối Cảnh Kỹ Thuật: Sự Khác Biệt Giữa Zalo PC/Web và Zalo Mobile

Trong hệ sinh thái Zalo:
1. **Zalo PC / Web (`chat.zalo.me`)**:
   * Được thiết kế thuần túy cho nhu cầu nhắn tin (Chat/Work). VNG **cắt bỏ** tính năng Nhật ký bài viết bạn bè và Zalo Video trên phiên bản này.
   * Session Web (`zpw_sek`) chỉ được cấp phép đọc các API cơ bản: hồ sơ cá nhân (`/api/social/profile/me-v2`), danh bạ (`/api/social/friend/getfriends`), hồ sơ bạn bè (`/api/social/friend/getprofiles`), và kho ảnh đại diện của chính mình (`/api/social/avatar-list`).
   * Máy chủ Web hoàn toàn không có endpoint REST nào để lấy feed bài đăng trên tường nhà của người khác.
2. **Zalo Mobile (Android / iOS)**:
   * Quản lý dòng thời gian chung, tường cá nhân bạn bè, Zalo Video, Khoảnh khắc (Story 24h).
   * Cơ chế truyền thông chính: **Kênh Socket nhị phân TCP liên tục (`NativeSocket`)**, mã lệnh **CMD `50001`**.
   * Cơ chế lưu trữ offline: Bộ đệm SQLite/Room cục bộ (`user_feed`, `timeline_feed_1`) giúp bài viết hiển thị tức thì mà không cần tải lại qua mạng nếu không có bài mới.

---

## 2. Kiến Trúc Nhật Ký Mobile (`com.zing.zalo.social.*`)

### 2.1 Các Tầng Xử Lý (Clean Architecture)
* **Domain Layer (UseCases)**:
  * `LoadFeedTimelineUseCase` (`rx0.y`): Tải dòng thời gian chung của bạn bè.
  * `LoadUserTimelineUseCase` (`nx0.b`): Tải dòng thời gian tường cá nhân của một bạn bè cụ thể (`ownerId`).
  * `InitTimelineUseCase` (`rx0.h`): Khởi tạo trạng thái và đọc bộ nhớ đệm ban đầu từ SQLite.
  * `PostFeedUseCase`: Tạo bài viết mới (status, nhiều ảnh, video, nhạc Zing MP3, sticker).
  * `UnTagFeedUseCase`, `HideOrReportAdsUseCase`.
* **Repository & Worker Layer**:
  * `bp.j0`: Network Repository quản lý toàn bộ các tác vụ mạng của Feed.
  * `nu0.v`: Network Worker phụ trách `fetchNewFeed` (làm mới) và `loadFeedPage` (phân trang).
  * `mw0.c`: Worker phụ trách bóc tách chi tiết Feed của một bạn bè (`FeedDetailBusiness`).
  * `o41.g`: Worker xử lý nghiệp vụ đăng bài mới (`PostFeedBusiness`).
* **UI & Presentation (`com.zing.zalo.ui.*`)**:
  * `TimelineView`, `TimelineFragment`: Khung giao diện dòng thời gian.
  * `TimelineFeedItemFooterBarV3`, `FeedInteractionBarUIV3`: Thanh tương tác (Like, Comment, Share).
  * `FeedReactionBottomSheet`: Bảng chọn emoji cảm xúc động.

### 2.2 Cơ Sở Dữ Liệu Cục Bộ (SQLite & Room Database)
Toàn bộ bài viết khi tải về máy đều được lưu cố định vào SQLite để phục vụ chế độ Offline-First:
* **`user_feed`**:
  * Lưu trữ bài viết trên trang cá nhân của bạn bè: `SELECT * FROM user_feed WHERE cUid = ?`.
* **`timeline_feed_1`**:
  * Bảng lưu trữ toàn bộ payload JSON bài đăng của dòng thời gian chung theo `(cUID, id)`.
* **`timeline_order_2`**:
  * Quản lý thứ tự bài viết hiển thị theo thời gian trên timeline.
* **`SnapshotFeedTable`**:
  * Lưu trữ bản nháp bài đăng khi người dùng soạn thảo chưa bấm gửi.

---

## 3. Đặc Tả Giao Thức Mạng: Lệnh Nhị Phân Socket `50001`

Dịch ngược trực tiếp từ các class `pw0.e.d`, `nu0.v.j`, `mw0.c.k` và bộ tuần tự hóa `uc.s`:

### 3.1 Bảng Mã Lệnh Mạng của Feed

| Nghiệp Vụ | Socket Command (Ưu tiên) | Subcommand (4B Big-Endian) | HTTP Fallback Endpoint | Host |
|---|---|---|---|---|
| **Xem Feed Chung** | CMD `50001` (`0xC351`) | `0x00150102` (`1,376,514`) | `GET /api/feed?` (Cmd `1057`) | `https://api-talk.api.zaloapp.com` |
| **Xem Feed Bạn Bè** | CMD `50001` (`0xC351`) | `0x00150102` (`1,376,514`) | `GET /api/feed?` (Cmd `1057`) | `https://api-talk.api.zaloapp.com` |
| **Đăng bài mới** | CMD `50001` (`0xC351`) | `0x00220411` (`2,229,265`) | `POST /api/feed/` (Cmd `1058`) | `https://api-talk.api.zaloapp.com` |
| **Sửa bài viết** | CMD `50001` (`0xC351`) | `0x26580022` (`643,301,410`) | `POST /api/feed/` (Cmd `1058`) | `https://api-talk.api.zaloapp.com` |
| **Upload Ảnh** | *(HTTP Multipart)* | — | `POST /api/photo?` (Cmd `1058`) | `https://photo.talk.zing.vn` |

### 3.2 Cấu Trúc Nhị Phân Của Gói Tin Xin Feed (`GET_FEED`)
Gói tin được đóng gói bởi `pw0.e.d` và `mw0.c.k`:

```
+-------------+-----------------------+-------+------------------------------------------+
| Offset (B)  | Tên trường            | Kiểu  | Giá trị / Ý nghĩa                        |
+-------------+-----------------------+-------+------------------------------------------+
| 0x00        | header_magic          | u8    | 0x01 (Phiên bản gói nhị phân)            |
| 0x01..0x04  | feed_version          | i32   | 0x00000002 (Version schema của feed)     |
| 0x05..0x08  | tab                   | i32   | 0: Tab Chính (Main), 1: Tab Khác (Other) |
| 0x09..0x0C  | page                  | i32   | Số trang (bắt đầu từ 1)                  |
| 0x0D..0x10  | count                 | i32   | Số bài viết cần lấy (mặc định 20 / 0x14) |
| 0x11..0x18  | last_feed_id_server   | i64   | ID bài cuối nhận ở lần trước (0 nếu mới) |
| 0x19..0x20  | last_feed_id_local    | i64   | ID bài mới nhất trong SQLite máy         |
| 0x21..0x28  | owner_id              | i64   | UID của bạn bè cần xem (0 nếu xem chung) |
| 0x29..0x30  | viewer_id             | i64   | UID của tài khoản bạn đang đăng nhập     |
| 0x31..0x32  | filter_flags          | i16   | Cờ bộ lọc (bạn thân, ẩn quảng cáo)       |
+-------------+-----------------------+-------+------------------------------------------+
```
Đóng gói vào frame:
$$\text{Frame} = [\text{Total\_Length (4B)}] + [\text{CMD: } 50001\text{ (2B)}] + [\text{SubCMD: } 0x00150102\text{ (4B)}] + [\text{Payload}]$$

### 3.3 Cấu Trúc Nhị Phân Của Gói Tin Đăng Bài (`POST_FEED` - `o41.g`)
1. `Subcommand`: `0x00220411` (`i32`).
2. `feedContentId`: Chuỗi UUID ngẫu nhiên định danh phiên đăng bài từ client.
3. `privacy`: `i32` (`0`: Công khai, `1`: Bạn bè, `2`: Chỉ mình tôi, `3`: Tùy chỉnh danh sách).
4. `message`: Nội dung văn bản bài viết (chuỗi UTF-8).
5. `media_count`: Số lượng ảnh/video đính kèm (`i16`).
   * Vòng lặp từng media: `photoId`, `url`, `width`, `height`, `type` (`1`: Ảnh, `2`: Video).
6. `linkAttachment`: Link web đính kèm nếu có (`mLinkSource`, `mLinkDescription`).
7. `logoMiniApp`: ID app nếu share từ Mini App Zalo.

### 3.4 Quy Trình Upload Ảnh Mạng Xã Hội (`/api/photo?`)
* **Host**: **`https://photo.talk.zing.vn`** (Command ID `1058`).
* **Header**: `Content-Type: multipart/form-data; boundary=ZiNgMeMoBiLe`.
* **Body Form**: Binary của file ảnh + `album_id` + `session_key`.
* **Phản hồi**: Server trả về `photoId` và link CDN ảnh mạng xã hội:
  `https://zpsocial2-f9-org.zadn.vn/<hash>.jxl`.

---

## 4. Dịch Ngược Tầng Bảo Mật Native (`libnative_utils.so`)

Toàn bộ logic ký số và định danh phần cứng nằm trong thư viện native C++ `libnative_utils.so` (ARM64) nhằm bảo vệ gateway di động `api-talk.api.zaloapp.com`:

### 4.1 Thuật Toán Sinh Chữ Ký `CoreUtils.generateSignatureNative` (`0x000279ec`)
Dịch ngược mã máy ARM64:
1. Đọc chuỗi tham số query đầu vào qua JNI `GetStringUTFChars`.
2. Khởi tạo ngữ cảnh băm `MD5_Init(ctx)` tại `0x51b60`.
3. Băm nội dung query string: `MD5_Update(ctx, utf_chars, len)`.
4. Nạp con trỏ salt từ biến toàn cục `0x65ae8` trong section `.bss` (hàm `0x280b0` và `0x284a0`).
5. Băm nối tiếp salt: `MD5_Update(ctx, salt, strlen(salt))`.
6. Kết thúc băm: `MD5_Final(digest, ctx)`.
7. Vòng lặp 16 vòng chuyển đổi raw digest sang chuỗi hex 32 ký tự thường (`lowercase hex`).
$$\text{sig} = \text{MD5}(\text{queryString} + \text{salt})$$

### 4.2 Cơ Chế Tự Bảo Vệ Tiến Trình (Anti-Tamper `/proc/self/cmdline`)
* Hàm khởi tạo cấu hình `0x000288e4` gọi trực tiếp Linux ARM64 syscall `openat` (`0x38`) vào `/proc/self/cmdline` để kiểm tra tên tiến trình.
* Nếu không đúng là gói `com.zing.zalo`, biến con trỏ salt giữ giá trị `NULL`, từ chối tạo chữ ký hợp lệ.

### 4.3 Định Danh Thiết Bị `CoreUtils.getZCIDNative` (`0x00027e44`)
* Đọc chuỗi ZCID phần cứng tại offset `+0x40` từ struct toàn cục.
* Giá trị này được gán vào cookie `zcid=<zcid_value>` trong mọi giao tiếp mạng.

---

## 5. Dịch Ngược Kiến Trúc Zalo Video (`https://api.zalo.video`)

Zalo Video là nền tảng video ngắn độc lập (tương tự TikTok/Reels) lồng trong Zalo:
* **Hệ thống Server**:
  * `https://api.zalo.video`: REST API lấy feed video, trending, thông tin channel, tương tác (Like, Comment, Follow).
  * `https://uploadapi.zalo.video`: Upload gateway cấp phát tạm thời AWS S3 STS (`S3TempCredential`) để client upload thẳng video lên bucket S3.
  * `wss://wss-core.zalo.video`: WebSocket thời gian thực cho livestream và video.
* **Xác thực OAuth 2.0**:
  * Khi gọi API video bằng cookie chat Web `zpw_sek`, gateway từ chối với mã lỗi `{"err": -106, "msg": "Tính năng không hỗ trợ"}`.
  * Cần gửi request lên `https://oauth.zaloapp.com` để đổi lấy `OauthToken.access_token` riêng biệt, sau đó truyền vào header `Authorization: Bearer <access_token>`.

---

## 6. Phân Tích Cơ Chế Quyền Riêng Tư & Bộ Nhớ Đệm (Privacy & Cache)

1. **Chính Sách Giới Hạn Người Xem (Privacy Filter)**:
   * Khi người dùng thiết lập giới hạn thời gian xem (ví dụ chỉ xem trong 6 tháng / 1 năm) hoặc chặn xem Nhật ký, máy chủ lọc ngay tại gateway và không trả về bài viết cũ cho các phiên kết nối mới.
2. **Cơ Chế Local Cache SQLite**:
   * App di động áp dụng mô hình Offline-First. Các bài viết đã từng được tải về máy trước khi quyền riêng tư bị thay đổi sẽ được lưu cứng trong bảng `user_feed`.
   * Khi người dùng mở lại trang cá nhân, app đọc trực tiếp từ cache cục bộ nên bài viết cũ vẫn hiển thị trên màn hình điện thoại mà không phát sinh thêm request tải nội dung qua mạng.

---

## 7. Hướng Dẫn Tích Hợp Vào Dự Án PC (`zalo-rust`)

Để đưa tính năng Nhật ký lên `zalo-rust` một cách bền vững và bảo mật:

### 7.1 Module Dữ Liệu Feed Chuẩn (`zalo-rust/test_lab/feed_model.rs`)
Định nghĩa đầy đủ các struct dữ liệu:
```rust
pub struct FeedAuthor {
    pub uid: String,
    pub display_name: String,
    pub avatar_url: String,
}

pub struct FeedMedia {
    pub photo_id: String,
    pub media_type: String, // "image" | "video"
    pub url: String,
    pub thumbnail: String,
    pub width: u32,
    pub height: u32,
}

pub struct FeedItem {
    pub feed_id: String,
    pub author: FeedAuthor,
    pub date_str: String,
    pub content: String,
    pub status_badge: Option<String>,
    pub media_list: Vec<FeedMedia>,
    pub reaction: FeedReaction,
    pub comments: Vec<FeedComment>,
}

pub struct UserTimelineProfile {
    pub uid: String,
    pub display_name: String,
    pub avatar_url: String,
    pub cover_url: String,
    pub bio: String,
    pub sdob: String,
    pub feeds: Vec<FeedItem>,
}
```

### 7.2 Danh Mục Script Thử Nghiệm Trong `test_lab/`
* `test_lab/01_test_web_social_suite.py`: Khai thác toàn diện các Web Social API (Profile, Friends, Cover, Avatar-list).
* `test_lab/02_test_ws_feed_command.py`: Thử nghiệm WebSocket và phân tích lỗi `3002 NOT_FOUND_WORKER`.
* `test_lab/feed_model.rs`: File model Rust với dữ liệu cấu trúc mẫu ẩn danh.
