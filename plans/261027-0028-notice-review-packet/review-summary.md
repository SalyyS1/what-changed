# Hồ sơ duyệt giấy phép cho installer hiện tại

**Trạng thái: đã chuẩn bị và kiểm tra kỹ thuật; chưa được con người duyệt.**
Ngày chuẩn bị: 2026-10-27. Người chuẩn bị: Codex.

Quyết định cần duyệt: chấp nhận các lựa chọn giấy phép, bản quyền/NOTICE bổ sung,
cách cung cấp mã nguồn MPL và chứng cứ nguồn gốc native trong bộ hồ sơ này để
ghi nhận người duyệt, tạo notices qua công cụ hiện có và build installer để kiểm thử.
Quyết định này chưa cho phép công bố bản phát hành và chưa xác nhận installer đạt QA.

## Phạm vi và bằng chứng

- 232 thư viện: 213 Cargo, 19 frontend; thêm 2 thành phần native.
- [Manifest đề xuất](proposed-manifest.json): từng phiên bản, lựa chọn, lý do,
  đường dẫn nguồn, bản quyền, NOTICE và mã băm.
- [Bản notices xem trước](NOTICE-PREVIEW-NOT-APPROVED.txt): toàn bộ văn bản sẽ
  được đưa vào bước tạo notices sau khi duyệt. Có nhãn chưa phê duyệt rõ ràng.
- [Kết quả kiểm tra](technical-validation.json): 263 văn bản tham chiếu khớp
  SHA-256; mọi mục đều còn `obligationsReviewed: false`.
- [Đối chiếu archive](archive-correspondence.json): 213 archive Cargo khớp
  checksum Cargo.lock; 10.116 file đóng gói khớp byte với source đang dùng.
  Reviewer kỹ thuật kiểm tra ledger và manifest, không chạy lại việc giải nén
  đối chiếu toàn bộ 10.116 file lần thứ hai.
- [Chứng cứ bổ sung](supplemental-evidence.json): nguồn và phép chuyển đổi của
  các văn bản mới; không tự điền bản quyền của package khác.
- [Các input đã khóa](inputs.json), [so sánh với draft cũ](comparison.json).
  Không thay đổi danh sách package hoặc nguồn đã ghi nhận. Raw Cargo metadata
  mới thay thế snapshot cũ; không thể giải thích chính xác khác biệt khi raw
  metadata cũ không còn. Raw chứa đường dẫn máy nằm riêng trong `.qa/`.
- [SHA256SUMS.txt](SHA256SUMS.txt) khóa các file của bộ hồ sơ và toàn bộ văn bản
  được tham chiếu; đường dẫn trong đó tính từ root repository.

## Lựa chọn đề xuất

Với giấy phép có lựa chọn thay thế, dùng một lựa chọn được khai báo và có toàn
bộ văn bản tương ứng. Với biểu thức AND, giữ tất cả điều kiện và văn bản.

| Giấy phép đề xuất | Số thư viện |
|---|---:|
| MIT | 202 |
| Unicode-3.0 | 15 |
| BSD-3-Clause | 4 |
| Apache-2.0 | 3 |
| BSD-3-Clause AND MIT | 2 |
| CC0-1.0 | 2 |
| Apache-2.0 AND MIT | 1 |
| MPL-2.0 | 1 |
| Zlib | 1 |
| BSL-1.0 | 1 |

Các quyết định cần chú ý:

1. **siphasher:** giữ nguyên COPYING chứa bản quyền và tuyên bố cho chọn MIT
   hoặc Apache; đề xuất MIT kèm toàn bộ điều khoản MIT tiêu chuẩn. Văn bản bổ sung
   được ghi đúng là bổ sung, không giả là file upstream bị thiếu. Đối chiếu với
   [MIT chuẩn](https://opensource.org/license/mit).
2. **brotli-decompressor:** giữ BSD-3-Clause của crate và phần MIT có bản quyền
   Google trong `src/context.rs`; bổ sung điều khoản MIT tiêu chuẩn. Đề xuất áp
   dụng cả hai, không dùng dấu `/` cũ để bỏ phần MIT.
3. **option-ext 0.2.0 / MPL-2.0:** không sửa source; giữ nguyên thông báo và cung
   cấp [archive công khai đúng phiên bản](https://static.crates.io/crates/option-ext/option-ext-0.2.0.crate).
   [Bản archive trong hồ sơ](sources/option-ext-0.2.0.crate), 7.345 byte,
   SHA-256 `04744f49eae99ab78e0d5c0b603ab218f515ea8cfe5a456d7629ad883a3b6e7d`,
   khớp Cargo.lock, source cục bộ và bản tải công khai. Giữ route này trong
   notices; nếu sửa source hoặc route không còn thì phải cập nhật cách cung cấp
   source trước khi phát hành. Căn cứ: [MPL 2.0, mục 3](https://www.mozilla.org/en-US/MPL/2.0/).
4. **svelte-toolbelt:** metadata không khai báo license; file LICENSE thực tế
   chứa MIT và bản quyền hai tác giả. Null metadata vẫn được giữ trong bằng chứng.
5. **dunce:** chọn CC0-1.0, là lựa chọn có đầy đủ văn bản đóng gói.
6. Giữ thêm bản quyền Unicode trong regex-syntax, spin trong tracing-core và
   magic-string lồng trong Svelte. Mục magic-string là inclusion thận trọng;
   không tuyên bố compiler/tooling được ship trong app. Các NOTICE/COPYRIGHT
   đã thu thập được giữ nguyên, gồm các biểu thức AND của Brotli và dpi. Giữ thêm
   AUTHORS từ cùng commit UNIC đã xác minh, được COPYRIGHT của năm crate dẫn tới;
   các liên kết trong văn bản upstream được giữ nguyên như nguồn.

## Hai thành phần native

| Thành phần | Đề xuất | Chứng cứ |
|---|---|---|
| SQLite 3.53.2 | Public domain, giữ nguyên thông báo amalgamation | SHA-256 source khớp; SHA3 từng đối chiếu trang release chính thức |
| WebView2Loader SDK 1.0.3650.58 | BSD-3-Clause, giữ cả LICENSE và NOTICE SDK | Version resource và ba native candidate khớp archive NuGet đúng phiên bản |

Chi tiết trong [báo cáo nguồn native](../reports/qa-261005-native-notice-source-correspondence.md).
Phần nguồn của báo cáo cũ được tái sử dụng sau đối chiếu input mới; trạng thái
snapshot cũ trong báo cáo không được coi là đã giải thích xong.

SQLite có feature `bundled`; không có feature SQLCipher. Source SQLCipher có
trong crate nhưng không được chọn, nên license đó không thay thế thông báo
public domain của amalgamation đang dùng. Quyền đối với deliverable SQLite được
mô tả tại [trang copyright chính thức](https://sqlite.org/copyright.html).

WebView2 SDK và Runtime có phạm vi khác nhau. Bộ hồ sơ này duyệt nguồn SDK,
không tự xác nhận điều kiện phân phối Runtime; chế độ installer và artifact
cụ thể còn cần kiểm tra theo
[tài liệu Microsoft](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution).
Duyệt native bao gồm lựa chọn terms/NOTICE và provenance, không chỉ cờ top-level.

## Sau quyết định duyệt

Ghi lại đúng người duyệt, thời điểm và phạm vi; tạo notices bằng generator đang
có; build installer; khóa mã băm executable/installer và kiểm tra bản cài đó.
Nếu inputs/source thay đổi ngoài phạm vi đã duyệt, đối chiếu lại trước khi dùng
quyết định cũ. Không có kết quả installer, chữ ký, linked coverage, acceptance
native, soak bốn giờ hay quyền công bố nào được suy ra từ bộ hồ sơ source này.

Yêu cầu có người duyệt nằm tại
[prepare-third-party-review.mjs](../../scripts/prepare-third-party-review.mjs):
“A human still chooses licenses, checks native components and NOTICE obligations”.
Generator hiện có cũng đòi `status: reviewed`, người duyệt và review các nghĩa vụ.
Codex đã làm phần thu thập, đề xuất và kiểm tra; các cờ phê duyệt vẫn để trống.

Còn chờ: quyết định của người duyệt cho bộ hồ sơ đã khóa; sau đó là artifact và
kiểm thử installer. Manifest phát hành và draft cũ vẫn nguyên trạng pending.
