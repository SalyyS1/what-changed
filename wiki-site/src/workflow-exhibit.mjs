import { booking, bookingNodeCopy } from './demo-data.mjs';
import { escapeHtml as e } from './markdown.mjs';

export const tourSteps = [
  { id: 'intent', en: ['Intent', 'Add rescheduling.', 'Staff should be able to move a booking to an available time.'], vi: ['Ý định', 'Thêm tính năng đổi lịch.', 'Nhân viên có thể chuyển lịch hẹn sang khung giờ còn trống.'] },
  { id: 'rule', en: ['Preserve', 'Keep customer access unchanged.', 'Only staff can reschedule. Keep the booking reference and staff assignment.'], vi: ['Giữ lại', 'Giữ đúng quyền khách hàng.', 'Chỉ nhân viên được đổi lịch. Giữ mã lịch hẹn và nhân viên phụ trách.'] },
  { id: 'rule', en: ['Brief', 'Take a clear brief to your AI.', 'The outcome, existing rules, and expected checks travel together as reviewed Markdown.'], vi: ['Brief', 'Đưa yêu cầu rõ ràng cho AI.', 'Mục tiêu, quy tắc đã xác nhận và tiêu chí kiểm tra đi cùng nhau trong Markdown.'] },
  { id: 'create', en: ['Verify', 'Check both sides of the rule.', 'Staff can move a booking. A customer cannot. These two observations belong to booking-demo-b.'], vi: ['Kiểm tra', 'Kiểm tra cả hai phía của quy tắc.', 'Nhân viên đổi được lịch; khách hàng không được phép. Hai kết quả thuộc bản booking-demo-b.'] },
  { id: 'readback', en: ['Decide', 'A passed check is not your decision.', 'Review the scope and gaps, then accept, reject, or defer. This example has no owner decision yet.'], vi: ['Quyết định', 'Test đạt vẫn cần bạn quyết định.', 'Xem phạm vi và phần còn thiếu rồi nhận, từ chối hoặc để sau. Ví dụ chưa có quyết định của chủ app.'] },
];

/** A real graph remains readable before the optional renderer has loaded. */
export function evidencePoster(vi = false) {
  return `<svg class="evidence-poster" viewBox="0 0 600 390" role="img" aria-label="${vi ? 'Ý định nối với hai kịch bản kiểm tra và kết quả quan sát' : 'Owner intent connects to two test scenarios and their observed results'}"><g class="poster-platforms"><path d="M30 50L470 24L570 96L130 122Z"/><path d="M30 170L470 144L570 216L130 242Z"/><path d="M30 290L470 264L570 336L130 362Z"/></g><g class="poster-links"><path d="M170 84H430M430 84L170 204M430 84V204M170 204V324M430 204V324"/></g>${booking.nodes.map((node, index) => `<g data-poster-node="${node.id}" class="poster-node${index === 0 ? ' is-current' : ''}"><rect x="${index % 2 ? 326 : 66}" y="${52 + node.layer * 120}" width="208" height="64" rx="6"/><text x="${index % 2 ? 430 : 170}" y="${79 + node.layer * 120}">${e(bookingNodeCopy(node, vi ? 'vi' : 'en').kind.split(' · ')[0])}</text><text class="poster-title" x="${index % 2 ? 430 : 170}" y="${99 + node.layer * 120}">${e(bookingNodeCopy(node, vi ? 'vi' : 'en').label)}</text></g>`).join('')}</svg>`;
}

export function workflowExhibit(vi = false) {
  const first = tourSteps[0][vi ? 'vi' : 'en'];
  return `<section class="workflow-exhibit" data-workflow data-language="${vi ? 'vi' : 'en'}" aria-label="${vi ? 'Quy trình thay đổi app minh họa' : 'Interactive app change walkthrough'}">
    <div class="exhibit-heading"><span>${vi ? 'Một thay đổi trong app booking' : 'One change in a booking app'}</span><span class="exhibit-format" data-model-status>${vi ? 'Sơ đồ 2D' : '2D map'}</span></div>
    <div class="exhibit-viewport"><div data-model-poster>${evidencePoster(vi)}</div><div class="exhibit-canvas" data-model-canvas hidden aria-label="${vi ? 'Bằng chứng lịch hẹn trong ba lớp không gian' : 'Booking evidence in three spatial layers'}"></div><div class="model-tools" data-tour-enhanced hidden><button type="button" data-model-toggle aria-pressed="false">3D</button><button type="button" data-model-rotate aria-label="${vi ? 'Xoay mô hình' : 'Rotate model'}" hidden>↻</button><button type="button" data-model-reset aria-label="${vi ? 'Đặt lại góc nhìn' : 'Reset model view'}" hidden>⤢</button></div></div>
    <div class="exhibit-caption"><span>${vi ? 'Ý định / kịch bản / kết quả' : 'Intent / scenarios / results'}</span><span>${vi ? 'Dữ liệu minh họa' : 'Synthetic data'}</span></div>
    <div class="tour-stepper" data-tour-enhanced hidden role="group" aria-label="${vi ? 'Chọn bước' : 'Choose a workflow step'}">${tourSteps.map((step, index) => `<button type="button" data-tour-step="${index}" aria-pressed="${index === 0}"><span>0${index + 1}</span>${step[vi ? 'vi' : 'en'][0]}</button>`).join('')}</div>
    <div class="tour-explanation" aria-live="polite"><strong data-tour-title>${first[1]}</strong><p data-tour-detail>${first[2]}</p></div>
    <p class="model-message small" data-model-message role="status"></p>
    <noscript><ol class="tour-static">${tourSteps.map((step) => `<li><strong>${step[vi ? 'vi' : 'en'][0]}.</strong> ${step[vi ? 'vi' : 'en'][2]}</li>`).join('')}</ol></noscript>
  </section>`;
}
