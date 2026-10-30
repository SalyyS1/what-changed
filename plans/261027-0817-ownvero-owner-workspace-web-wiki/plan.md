---
title: "Ownvero — Owner workspace, visual evidence, landing and web wiki"
description: "Kế hoạch chuyển từ lịch sử thay đổi sang làm chủ app AI: yêu cầu, kiểm chứng, quyết định, bộ nhớ dự án, 2D/3D và website tải app."
status: in-progress
priority: P1
effort: 432h
branch: codex/ownvero-owner-workspace
tags: [feature, frontend, backend, database, docs, experimental]
blockedBy: []
blocks: []
created: 2026-10-27
---

# Ownvero — kế hoạch triển khai

## Overview

Mode: Deep / HOLD SCOPE. Đang triển khai trên nhánh riêng; chưa xuất bản.
G0/G1 đã được user duyệt, ghi tại [approval ledger](approval-ledger.json).
User cho phép triển khai hết tính năng rồi kiểm chứng giới hạn sau, theo
[quyết định mới](reports/approval-feature-first-sequence.json), thay thứ tự
[P02–P06 trước đây](reports/approval-offline-sequence.json). Windows sạch,
giới hạn mạng và người dùng thật vẫn là nghiệm thu bắt buộc, chuyển sang đợt sau.
G2/G3 chưa được duyệt. Không coi chỉ đạo “làm hết” là quyền vượt các gate này.

**Giá trị:** mỗi lần sửa app bằng AI, người dùng biết cần giữ gì, đã kiểm tra gì,
còn thiếu gì và tự quyết định nhận thay đổi. Không phải một AI IDE hay điểm “an toàn”.
Kiến trúc đề xuất: app Windows local-first + runner opt-in; website public static,
không phải SaaS đăng nhập. Xem [quyết định và phạm vi](decision-contract.md).

## Goals

1. My app → yêu cầu thay đổi → brief → kiểm chứng → chủ app chấp nhận → lần sửa tiếp.
2. Quy tắc được owner duyệt, bằng chứng gắn phiên bản; export không giữ người dùng làm con tin.
3. 2D và 3D giải thích luồng, quan hệ và bằng chứng; cùng dữ liệu, có giao diện danh sách đầy đủ.
4. Brand Ownvero trầm, landing rõ giá trị, wiki web có tìm kiếm, download đúng artifact.

## Phases

| # | Phase | Depends | Effort |
|---|-------|---------|--------|
| 01 | [Khóa phạm vi, đối chiếu repo, feasibility gates](phase-01-start.md) | — | 20h |
| 02 | [Hợp đồng kiến trúc và đường chuyển đổi](phase-02-contract-and-migration.md) | 01 | 24h |
| 03 | [Domain, persistence và typed IPC](phase-03-owner-workspace-domain.md) | 02 | 40h |
| 04 | [Luồng My app và Change request](phase-04-owner-workspace-flows.md) | 03 | 40h |
| 05 | [Brief, bộ nhớ và portable export/import](phase-05-portable-brief-and-memory.md) | 04 | 28h |
| 06 | [Kịch bản, bằng chứng, owner acceptance](phase-06-acceptance-scenarios-and-evidence.md) | 05 | 40h |
| 07 | [Runner kiểm chứng có cấp quyền](phase-07-authorized-preview-runner.md) | 06, G1 | 64h |
| 08 | [Mô hình 2D có tương tác](phase-08-meaningful-2d-views.md) | 07 | 28h |
| 09 | [Mô hình 3D và tương đương truy cập](phase-09-meaningful-3d-view.md) | 08 | 32h |
| 10 | [Brand production và app shell](phase-10-ownvero-brand-and-app-shell.md) | 09, G2 | 24h |
| 11 | [Landing, wiki web, demo, download](phase-11-landing-wiki-and-download.md) | 10 | 44h |
| 12 | [Hardening, installer, pilot và phát hành](phase-12-verification-release-and-pilot.md) | 11; G3 chỉ publish | 48h |

P02 hoàn tất hợp đồng thiết kế. P01 và P03–P06 đang triển khai/kiểm chứng;
nghiệm thu bản cài và người dùng thật vẫn còn mở.
P07–P12 tiếp tục triển khai theo thứ tự mới. [Nhật ký triển khai offline](reports/offline-execution.md)
ghi source, kiểm thử và phần còn thiếu.

Source tiếp tục đã có runner opt-in, ảnh được duyệt từng lần, ZIP test độc lập,
bản đồ bằng chứng 2D/list/3D và landing/wiki local 19 trang. Xem
[báo cáo tính năng](../reports/implementation-261028-0859-remaining-ownvero-features.md)
và [sổ nghiệm thu sau triển khai](reports/deferred-acceptance.md).
Các phase 07–12 giữ `in-progress`: source có mặt không đồng nghĩa đã qua
nghiệm thu native, giới hạn mạng, người dùng thật, G2 hoặc G3.

Luồng clipboard Markdown, Save/Open và import ZIP đã qua trên Tauri debug với
dữ liệu tổng hợp; hồ sơ gốc đã khôi phục. Xem
[báo cáo tiếp tục](../reports/qa-261028-0440-offline-continuation.md).
Full verify, hiệu năng, nghiệm thu installer và người dùng thật vẫn mở.

432h là ước lượng engineering, không phải cam kết lịch;
chưa gồm chờ phê duyệt, ký số và pilot 2 tuần. Dự phòng planning 432–650h;
runner spike ở P01 có thể buộc re-estimate. Pilot sớm ở P06 trước runner, không chờ tới launch.
Mỗi phase sau P01 phải scout lại owner/test/contract trước cook. Mặc định chạy tuần tự;
chỉ song song phần không chung file sau khi khóa interface. Không tự đổi scope.

## Success Criteria

- [ ] Owner hoàn thành một thay đổi thật và lặp lại với yêu cầu thứ hai, giữ đúng quy tắc cũ.
- [ ] Không có false pass cho thiếu quyền, build không rõ, evidence stale hoặc assertion bị giảm.
- [ ] 2D/3D/list cùng verdict, điều hướng keyboard, reduced-motion và WebGL fallback đạt gate.
- [ ] Landing/wiki/download dùng demo sạch, tài liệu đúng bản, artifact được duyệt mới cho tải.
- [ ] Dữ liệu/collector cũ không bị mất; native upgrade, privacy, notices và release gates có chứng cứ.

## Contracts và bằng chứng lập kế hoạch

- [Domain, trạng thái, evidence và API dự kiến](domain-evidence-contract.md)
- [Runner, dữ liệu nhạy cảm và giới hạn quyền](runner-safety-contract.md)
- [Preview đầu tiên: revision marker, test roles và backend adapter](supported-preview-contract.md)
- [UX, mô hình 2D/3D và thiết kế website](experience-and-site-contract.md)
- [Test matrix, ownership và cook protocol](verification-and-handoff.md)
- [Scout, research và giới hạn đã xác minh](research-and-source-map.md)
- [Red-team và validation](validation-review.md)

## Cross-plan / authorization

Plan cũ [What Changed MVP](../260829-0437-what-changed-desktop-mvp/plan.md) vẫn sở hữu
các gate chưa xong; P01 phân loại carry-forward, không đánh dấu xong bằng pivot.
G0: chốt desktop-primary; G1: network/runner/privacy mới; G2: public name/assets;
G3: frozen release approval. Chi tiết và điểm dừng trong decision-contract.md.

## Cook handoff

~~~text
/ak:cook D:/Project/.WhatChage/plans/261027-0817-ownvero-owner-workspace-web-wiki/plan.md
~~~

Chỉ đề xuất lệnh, chưa thực thi; không thêm --auto. P01 giải quyết gate trước khi code.

<!-- slug: ownvero-owner-workspace-web-wiki -->
