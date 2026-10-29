/** Synthetic only. Both renderers read these exact nodes and explicit edges. */
export const booking = Object.freeze({
  label: 'Booking example · synthetic data',
  before: 'booking-demo-a', after: 'booking-demo-b', environment: 'isolated demo',
  nodes: [
    { id: 'intent', layer: 0, kind: 'Capability', label: 'Reschedule a booking', detail: 'Move a booking to an available time. Keep the same booking reference and staff assignment.' },
    { id: 'rule', layer: 0, kind: 'Confirmed rule · r2', label: 'Only staff can reschedule', detail: 'A customer cannot change another person’s booking. This rule stays unchanged across both builds.' },
    { id: 'create', layer: 1, kind: 'Scenario · s3', label: 'Staff moves a booking', detail: 'Staff selects an available time. Read back the booking reference, new time, and staff assignment.' },
    { id: 'deny', layer: 1, kind: 'Scenario · s2', label: 'Customer request denied', detail: 'A customer tries the staff-only reschedule action. Expect denial and unchanged stored booking.' },
    { id: 'readback', layer: 2, kind: 'Observed result', label: 'Booking read-back', detail: 'Synthetic observation: booking BK-104 keeps its reference and staff assignment at the requested time.' },
    { id: 'denial', layer: 2, kind: 'Observed result', label: 'Authorization response', detail: 'Synthetic observation: the earlier build allowed a customer request; the candidate denies it.' },
  ],
  edges: [
    { from: 'intent', to: 'rule', type: 'requires' },
    { from: 'rule', to: 'create', type: 'verifies' },
    { from: 'rule', to: 'deny', type: 'verifies' },
    { from: 'create', to: 'readback', type: 'observed_by' },
    { from: 'deny', to: 'denial', type: 'observed_by' },
  ],
  states: {
    current: { label: 'Current observation', before: 'failed', after: 'passed', explanation: 'The candidate meets these two scenarios. Owner decision: not yet made. A pass covers the checked behavior only.' },
    failed: { label: 'Failed check', before: 'failed', after: 'failed', explanation: 'Expected: a customer request is denied. Observed: the booking changed. Review the evidence before requesting a repair.' },
    blocked: { label: 'Missing permission', before: 'failed', after: 'blocked', explanation: 'The staff test role is missing. No assertion was completed; blocked is not a pass. Configure an authorized test role first.' },
    stale: { label: 'Changed candidate', before: 'failed', after: 'stale', explanation: 'The result belongs to booking-demo-b, but booking-demo-c is now selected. Review the changed candidate and run again.' },
  },
});
