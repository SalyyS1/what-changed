import { button, url } from './layout.mjs';
import { workflowExhibit } from './workflow-exhibit.mjs';

export function landing(vi = false) {
  const copy = vi ? {
    label: 'Dành cho người làm app với AI', title: 'AI viết code.<br>Bạn làm chủ app.',
    intro: 'Giữ điều đang chạy tốt. Đưa AI yêu cầu rõ ràng. Xem bằng chứng trước khi nhận thay đổi.',
    try: 'Xem một lần sửa', download: 'Bản Windows',
    heading: 'Lần sửa sau, đừng bắt đầu lại.', memory: 'Trí nhớ cho app của bạn.',
    body: 'Quy tắc, yêu cầu và quyết định ở cùng dự án. Đổi cuộc chat, bạn vẫn giữ được điều quan trọng.',
    rows: [['Quy tắc', 'Chỉ nhân viên được đổi lịch'], ['Yêu cầu mới', 'Cho phép chuyển khung giờ'], ['Lần sửa tiếp', 'Thêm email nhắc lịch. Giữ nguyên quyền.']],
    brief: 'Yêu cầu rõ hơn. Ít đoán hơn.', briefBody: 'Đưa bản brief đã xem lại cho AI bạn đang dùng. Không cần đổi công cụ để giữ ngữ cảnh.',
    evidence: '“Đạt” cần có phạm vi.', evidenceBody: 'Đạt, lỗi, thiếu quyền, hay kết quả đã cũ: mỗi trạng thái nói rõ bước tiếp theo.',
    states: [['Đạt', 'Hai kịch bản đã được kiểm tra.'], ['Lỗi', 'Kết quả khác với điều mong đợi.'], ['Bị chặn', 'Thiếu quyền kiểm tra; chưa có kết luận.'], ['Đã cũ', 'App đã đổi bản; cần kiểm tra lại.']],
    end: 'Làm app với AI.<br>Giữ lại sự rõ ràng.', endBody: 'Workspace lưu tại máy. Brief Markdown mang đi được. Runner chỉ chạy khi bạn cấp quyền.',
    guide: 'Hướng dẫn nhanh', privacy: 'Dữ liệu & quyền riêng tư', unavailable: 'Windows x64 · Đang hoàn tất nghiệm thu · Chưa mở tải công khai',
  } : {
    label: 'For people building apps with AI', title: 'AI writes code.<br>You own the app.',
    intro: 'Keep what works. Give AI a clear brief. See the evidence before you accept the next change.',
    try: 'Explore a change', download: 'Windows app',
    heading: 'Next change. Same understanding.', memory: 'A memory for your app.',
    body: 'Rules, requests, and decisions stay with the project. Switch conversations without losing what matters.',
    rows: [['Keep', 'Only staff can reschedule'], ['Change', 'Let staff choose another time'], ['Next time', 'Add reminder emails. Keep the access rule.']],
    brief: 'A better brief. Less guesswork.', briefBody: 'Take reviewed Markdown to the coding AI you already use. Keep your context when you switch tools.',
    evidence: 'A pass needs a scope.', evidenceBody: 'Passed, failed, blocked, or stale. Each label tells you what the evidence can actually support.',
    states: [['Passed', 'These two scenarios were checked.'], ['Failed', 'The observation differs from the expectation.'], ['Blocked', 'Permission is missing. No verdict yet.'], ['Stale', 'The build changed. Run the checks again.']],
    end: 'Build with AI.<br>Keep your bearings.', endBody: 'A local workspace. Portable Markdown. A preview runner that starts only with your permission.',
    guide: 'Getting started', privacy: 'Data & privacy', unavailable: 'Windows x64 · Acceptance in progress · Public download not open yet',
  };
  return `<section class="product-hero wrap"><div class="hero-copy-block"><p class="eyebrow">${copy.label}</p><h1>${copy.title}</h1><p class="hero-copy">${copy.intro}</p><div class="actions">${button(copy.try, '#walkthrough')}${button(copy.download, 'download/', true)}</div></div><div id="walkthrough" class="hero-exhibit">${workflowExhibit(vi)}</div></section>
    <div class="wrap product-footnote"><p>${vi ? 'Ví dụ minh họa, không chạy test trên app thật của bạn.' : 'An interactive, synthetic example. This website does not test your app.'}</p><a href="${url('how-it-works/')}">${vi ? 'Toàn bộ quy trình' : 'The full workflow'} <span aria-hidden="true">↗</span></a></div>
    <section class="section wrap memory-section"><h2>${copy.heading}</h2><div class="memory-grid"><div class="memory-ledger" aria-label="${vi ? 'Bộ nhớ dự án minh họa' : 'Illustrative project memory'}"><div class="ledger-heading"><span>BOOKING APP</span><span>${vi ? 'Bộ nhớ dự án' : 'Project memory'}</span></div>${copy.rows.map(([label, text], index) => `<div class="ledger-row"><span class="ledger-index">0${index + 1}</span><div><span class="small">${label}</span><strong>${text}</strong></div>${index === 0 ? `<span class="ledger-lock" aria-label="${vi ? 'Quy tắc đã xác nhận' : 'Confirmed rule'}">✓</span>` : ''}</div>`).join('')}<div class="ledger-connection" aria-hidden="true">↳</div></div><div class="feature-copy"><h3>${copy.memory}</h3><p>${copy.body}</p><a class="text-link" href="${url('docs/owner-workspace/')}">${vi ? 'Xem workspace' : 'Inside the workspace'} <span aria-hidden="true">→</span></a></div></div></section>
    <section class="section wrap handoff-section"><div class="feature-copy"><h2>${copy.brief}</h2><p>${copy.briefBody}</p><a class="text-link" href="${url(vi ? 'vi/quickstart/' : 'docs/getting-started/')}">${copy.guide} <span aria-hidden="true">→</span></a></div><figure class="brief-sheet"><figcaption><span>change-request.md</span><span>MARKDOWN</span></figcaption><pre><code>${vi ? "# Cho nhân viên đổi lịch hẹn\n\n## Giữ nguyên\nChỉ nhân viên được đổi lịch.\nGiữ mã lịch hẹn và nhân viên phụ trách.\n\n## Kiểm tra\nNhân viên yêu cầu: đổi sang giờ đã chọn.\nKhách yêu cầu: từ chối, giữ nguyên lịch." : "# Let staff reschedule a booking\n\n## Keep\nOnly staff can reschedule.\nKeep booking reference and staff assignment.\n\n## Check\nStaff request: booking moves to the chosen time.\nCustomer request: denied, booking unchanged."}</code></pre></figure></section>
    <section class="section wrap evidence-section"><div class="feature-copy"><h2>${copy.evidence}</h2><p>${copy.evidenceBody}</p><a class="text-link" href="${url('demo/')}">${vi ? 'Thử các trạng thái' : 'Try the evidence states'} <span aria-hidden="true">→</span></a></div><div class="state-ledger">${copy.states.map(([state, text], index) => `<div><span class="evidence-state state-${index}"><span aria-hidden="true">${['✓', '×', '∅', '↺'][index]}</span>${state}</span><p>${text}</p></div>`).join('')}</div></section>
    <section class="release-section"><div class="wrap"><div><h2>${copy.end}</h2><p>${copy.endBody}</p><div class="actions">${button(copy.download, 'download/')}${button(copy.privacy, 'docs/privacy/', true)}</div><p class="small">${copy.unavailable}</p></div><div class="release-emblem" aria-hidden="true"><img src="${url('assets/mark.svg')}" width="160" height="160" alt=""></div></div></section>
    ${vi ? '' : legacyLinks()}`;
}

function legacyLinks() {
  return `<section class="wrap legacy-links" aria-label="Legacy wiki links"><h2>From the original What Changed?</h2>${[['overview', 'Overview', 'docs/legacy/'], ['first-investigation', 'Investigation', 'docs/legacy/'], ['timeline', 'Timeline', 'docs/legacy/#timeline'], ['workspace', 'Migration', 'docs/migration-guide/'], ['privacy', 'Privacy', 'docs/privacy/'], ['build', 'Build', 'docs/legacy/#build'], ['status', 'Release status', 'download/']].map(([id, label, path]) => `<a id="${id}" href="${url(path)}">${label}</a>`).join('')}</section>`;
}
