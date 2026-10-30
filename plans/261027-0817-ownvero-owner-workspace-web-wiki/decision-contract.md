# Decision contract — Ownvero

Status: đề xuất triển khai, 2026-10-27. Không phải quyền chạy hoặc xuất bản.
Root mọi đường dẫn trong plan: D:/Project/.WhatChage.
Nguồn ý tưởng: [brainstorm được user đồng ý về hướng](../reports/brainstorm-261027-1438-vibe-owner-product-direction.md).

## Outcome và phạm vi đủ

Người dùng non-tech đã có một web app chạy được, cần thêm/sửa tính năng liên tục,
và muốn người khác có thể dựa vào app đó. Ownvero giúp họ làm chủ yêu cầu và
nghiệm thu, không buộc hiểu git/diff trước khi nhận giá trị.

Phải giao đủ: My app, Change request, acceptance scenarios, bằng chứng, quyết định
owner, portable memory, kiểm chứng preview được cấp quyền, mô hình 2D + 3D trong app,
brand, landing + wiki web + demo + download, installer và pilot.
Không được gọi việc chỉ làm một dashboard hoặc đổi logo là hoàn tất pivot.

Không thuộc scope: hosted SaaS/accounts, remote LLM mặc định, AI tự sửa code,
IDE mới, deploy/rollback DB, production monitoring, billing, marketplace,
mọi provider/OS/browser, đồng bộ team hay tích hợp MCP tự động.
Đây là những tính năng chưa được yêu cầu, không phải cắt scope đã chấp thuận.

## Kiến trúc đề xuất và trade-off

| Lựa chọn | Lợi ích | Chi phí/rủi ro | Kết luận đề xuất |
|---|---|---|---|
| Desktop-primary + opt-in runner | Tận dụng Tauri/SQLite; không account; đúng yêu cầu tải app | Windows-only ban đầu; browser/runtime lớn hơn; đổi privacy/process promises | Dùng cho plan này, cần G0/G1 |
| Hosted workspace + desktop companion | Dễ thử từ trình duyệt, đa nền tảng | Backend/auth/hosting/secret custody mới; không có trong repo | Không lén đưa vào scope; cần plan khác nếu user chọn |
| Offline-only workspace | Giữ toàn bộ lời hứa cũ | Không thể chạy test remote; không giao được automated verification đã đề xuất | Không gọi là tương đương; nếu G1 bị từ chối phải replan |

Website public là sản phẩm truyền thông/tài liệu độc lập: không đăng nhập, không
lưu project, không chạy test trên URL khách nhập. Website không có Tauri IPC.
App core vẫn dùng được offline để lập brief, xem evidence cũ và export.
Một owner workspace có thể có preview được chọn rõ ràng, và liên kết project root
tùy chọn. Workspace troubleshooting cũ vẫn yêu cầu root, không đổi nghĩa âm thầm.

## Approval ledger và gate

| Gate | Quyết định cần ghi lại | Không được suy diễn | Điểm dừng |
|---|---|---|---|
| G0 | Desktop-primary Windows local-first cho bản đầu; website static | “Nice” không tự chốt lại đề xuất hosted workspace trước đó | P01 trước thay đổi user-visible architecture |
| G1 | Runner con có network theo phiên, runtime/browser đóng gói; dữ liệu test/evidence mới; thay lời hứa tuyệt đối cũ | Updater opt-in cũ không cấp quyền browser testing hoặc lưu auth | P01 thu quyết định; P02/P07 không đổi contract/chạy target trước khi có |
| G2 | Ownvero là tên public và production identity sau kiểm tra tên/asset | Exact-match search không phải trademark clearance | Trước public rename/assets trong P10 |
| G3 | Duyệt version/hash/manifest/notices/site claims của một release cụ thể | Source-publication hay notices cũ không duyệt binary mới | P12 trước upload binary hoặc chuyển nút Download sang available |

Phần chưa được user quyết định phải giữ nhãn proposed. Người thực thi hỏi một
câu rõ khi tới gate; không giả lập user answer, không dùng --auto để vượt quyền.
Chỉ G0/G1 là quyết định kiến trúc cần sớm; G2/G3 không cản nghiên cứu/domain local.

Machine-readable mirror: [approval-ledger.json](approval-ledger.json), hiện cả bốn
gate pending. required_approvals trong phase là contract của plan, chưa phải tính năng
được xác minh của ak CLI. P01 phải tạo guard check-ownvero-approvals.mjs, kiểm tra ledger,
scope digest và evidence trước dispatch; release/site commands phải gọi guard G3.
Guard không tự cấp quyền và không kiểm soát lệnh ngoài đường tích hợp. Trước khi guard
tồn tại, controller kiểm tra thủ công và chỉ cho phép read-only preflight.
Gate chưa đủ thì giữ phase pending, ghi lý do; không giả mạo status hoặc approval.

## Tương thích và các promise phải sửa có kiểm soát

- Giữ identifier dev.whatchanged.app, data path, registry uninstall identity và package/library
  identity trong bản chuyển đổi đầu. Tên hiển thị có thể đổi ở P10; tên binary chỉ đổi
  khi upgrade rehearsal chứng minh shortcut/autostart/uninstall vẫn đúng.
- Không đổi tên repo hoặc URL Pages trong plan này. Ownvero hiển thị trên route hiện có
  trước; vanity domain và repository rename là quyết định riêng.
- Giữ dữ liệu, collector preferences, watched folders, known-good pointer, autostart
  và legacy navigation. Không tự bật collectors khi thêm owner workspace.
- Khuyến nghị owner-workspace database riêng dưới cùng app-data root; không nhét
  business rules vào event/entity history. P02 spike phải chứng minh migration,
  purge-all, storage usage, backup/recovery và shutdown cùng quản lý cả hai.
- Database legacy không bị giả định có thể downgrade sau migration. Backup/restore
  có kiểm tra checksum; muốn restore phải app đóng và user xác nhận phần dữ liệu mới mất.
- “No runtime network / one process / no second runtime” chỉ còn đúng cho chế độ core
  nếu G1 được chấp thuận. Phải cập nhật README/ARCHITECTURE/PRIVACY/SECURITY/DECISIONS,
  installer disclosure, release gates và site cùng lúc; không chỉ xóa check-no-network.
- Core không có generic shell/HTTP/filesystem IPC; remote preview tuyệt đối không được
  mở trong webview có bridge. Runner không dùng browser profile cá nhân.
- User-authored business text và ảnh test là loại dữ liệu mới: có thể nhạy cảm.
  Không hứa bộ lọc text có thể bảo đảm xóa mọi bí mật khỏi ảnh/video.
- Browser profile tạm có thể lưu auth/cache nhạy cảm; phải công khai giới hạn này ở G1.
  Xóa profile không bảo đảm secure erase khỏi pagefile, backup hay thiết bị đã bị xâm nhập.

## Đường chuyển đổi giá trị

Legacy troubleshooting ở mục “Local diagnostics”; không còn là homepage mặc định
của người chọn Ownvero. Project root cũ có thể được owner liên kết vào My app,
nhưng không tự tạo “tính năng đã kiểm chứng” từ service/port/package.
Mark working tiếp tục tạo checkpoint + link workspace theo quyết định cũ.
Nút mới “Accept change” là quyết định nghiệp vụ khác; không đổi nghĩa Mark working.

## Milestones và điều kiện thành công

1. Sau P03–P05: offline owner workflow hoàn chỉnh, chạy được hai lượt thay đổi bằng copy/paste.
2. Sau P06–P07: một app booking/CRUD test thực, hai vai trò, deterministic assertions,
   thất bại/cancel/stale/missing permission đúng. Không chỉ demo UI.
3. Sau P08–P10: 2D/3D/list nói cùng sự thật; brand dùng được ở icon nhỏ/native.
4. Sau P11: website local preview hoàn chỉnh, docs/search/deep links hoạt động;
   chưa có binary được duyệt thì download unavailable có lý do.
5. Sau P12: frozen installer có native evidence; pilot lặp lại cho tín hiệu ích lợi.
   Pilot không phải chứng nhận product-market fit.

## Assumptions và kill signals

| Assumption | Kill signal | Response |
|---|---|---|
| Owner có môi trường test tách khỏi production | Preview dùng DB/payment/email thật mà không tách được | Không chạy mutation; blocked với hướng dẫn test environment |
| Packaged runner hoạt động không cần admin/Node của user | Fresh Windows chỉ chạy được khi dùng PATH/dev deps | Dừng P01/P07, đổi packaging trước tính tiếp |
| Owner xác nhận expected behavior đủ dễ | 4/10 pilot cần dev viết hộ kịch bản cơ bản | Sửa authoring/onboarding trước mở thêm feature |
| Evidence có build identity đáng tin cậy | Preview mutable không nhận diện được revision | “Observed at time”; không durable current-build acceptance |
| Sản phẩm đáng dùng lại | <5/10 trở lại vì một thay đổi thật | Re-evaluate thesis, không thêm notification để ép retention |

## Open questions

G0/G1 cần xác nhận trước triển khai tương ứng; G2/G3 có gate riêng. Pricing, domain
mua mới, macOS và hosted SaaS không được tự quyết trong cook.
