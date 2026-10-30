import type { LocatorDto, NamedAssertionDto, OriginDto, ReadBackAdapterDto, RecipeActionDto, RecipeAssertionDto, RecipeStepDto, ScenarioRecipeDto } from '../../lib/tauri/bindings';

export type ScenarioTemplate = 'booking-create' | 'booking-read-back' | 'booking-access';
export const blankLocator = (): LocatorDto => ({ kind: 'label', text: '' });
export function newAssertion(): NamedAssertionDto {
  return { id: crypto.randomUUID(), expected_behavior: '', assertion: { kind: 'visible', locator: blankLocator() } };
}
export function newRecipeStep(action: RecipeActionDto = { kind: 'navigate', path: '/' }): RecipeStepDto {
  return { id: crypto.randomUUID(), action, assertions: [] };
}

/** Template wording is a proposal. Only the owner's explicit confirmation persists it. */
export function scenarioTemplate(kind: ScenarioTemplate, origin?: OriginDto): { title: string; expected: string; recipe: ScenarioRecipeDto } | null {
  const recipe: ScenarioRecipeDto = { schema_version: 1, role_refs: [], fixture_refs: ['booking-test-data'], steps: [], adapters: [] };
  if (kind === 'booking-create') {
    recipe.role_refs = ['customer-test'];
    recipe.steps = [newRecipeStep({ kind: 'navigate', path: '/bookings' }),
      newRecipeStep({ kind: 'fill', locator: { kind: 'label', text: 'Customer name' }, fixture_field: 'customer_name' }),
      newRecipeStep({ kind: 'fill', locator: { kind: 'label', text: 'Booking time' }, fixture_field: 'booking_time' }),
      { ...newRecipeStep({ kind: 'click', locator: { kind: 'role', role: 'button', name: 'Book' } }), assertions: [{ id: crypto.randomUUID(), expected_behavior: 'The booking confirmation is visible for the selected test booking.', assertion: { kind: 'visible', locator: { kind: 'test_id', value: 'booking-confirmation' } } }] }];
    return { title: 'Customer creates a booking', expected: 'A customer can submit a booking using isolated test data and see its confirmation.', recipe };
  }
  if (!origin) return null;
  const adapterId = crypto.randomUUID();
  if (kind === 'booking-read-back') {
    recipe.role_refs = ['staff-test'];
    recipe.adapters = [{ id: adapterId, origin: { ...origin }, path_template: '/api/bookings/{fixture_id}', role_ref: 'staff-test', allowed_fields: ['customer_name', 'booking_time'], effect: 'read_only' }];
    recipe.steps = [{ ...newRecipeStep({ kind: 'read_back', adapter_id: adapterId }), assertions: [
      { id: crypto.randomUUID(), expected_behavior: 'The stored customer name matches the booking fixture.', assertion: { kind: 'field_equals', field: 'customer_name', fixture_field: 'customer_name' } },
      { id: crypto.randomUUID(), expected_behavior: 'The stored booking time matches the booking fixture.', assertion: { kind: 'field_equals', field: 'booking_time', fixture_field: 'booking_time' } },
    ] }];
    return { title: 'Staff reads the same booking', expected: 'A separate staff test role can read back the exact booking and matching fields.', recipe };
  }
  recipe.role_refs = ['other-customer-test'];
  recipe.adapters = [{ id: adapterId, origin: { ...origin }, path_template: '/api/bookings/{fixture_id}', role_ref: 'other-customer-test', allowed_fields: ['customer_name', 'booking_time'], effect: 'read_denied' }];
  recipe.steps = [{ ...newRecipeStep({ kind: 'deny_other_role', adapter_id: adapterId }), assertions: [{ id: crypto.randomUUID(), expected_behavior: 'A different ordinary customer role is denied access and no booking fields are disclosed.', assertion: { kind: 'denied', allowed_statuses: [403, 404], absent_fields: ['customer_name', 'booking_time'] } }] }];
  return { title: 'Another customer cannot read the booking', expected: 'The named booking route denies a separate customer test role without disclosing booking fields.', recipe };
}

export function describeLocator(locator: LocatorDto): string {
  if (locator.kind === 'role') return `${locator.role} named “${locator.name}”`;
  if (locator.kind === 'label') return `field labelled “${locator.text}”`;
  return `test ID “${locator.value}”`;
}

export function describeAssertion(assertion: RecipeAssertionDto): string {
  if (assertion.kind === 'visible') return `${describeLocator(assertion.locator)} is visible`;
  if (assertion.kind === 'text_equals') return `${describeLocator(assertion.locator)} exactly equals “${assertion.expected}”`;
  if (assertion.kind === 'field_equals') return `stored field “${assertion.field}” matches test field “${assertion.fixture_field}”`;
  return `denied with ${assertion.allowed_statuses.join(' or ')}; absent fields: ${assertion.absent_fields.join(', ') || '(none specified)'}`;
}

export function describeAction(action: RecipeActionDto, adapters: ReadBackAdapterDto[] = []): string {
  switch (action.kind) {
    case 'navigate': return `Open ${action.path}`;
    case 'fill': return `Fill ${describeLocator(action.locator)} from test field “${action.fixture_field}”`;
    case 'click': return `Activate ${describeLocator(action.locator)}`;
    case 'cleanup': return `Clean up with ${describeLocator(action.locator)}`;
    case 'read_back':
    case 'deny_other_role': {
      const adapter = adapters.find((item) => item.id === action.adapter_id);
      return `${action.kind === 'read_back' ? 'Read stored item' : 'Check denied access'}${adapter ? ` at ${adapter.origin.scheme}://${adapter.origin.host}:${adapter.origin.port}${adapter.path_template}, role “${adapter.role_ref}”, allowed fields: ${adapter.allowed_fields.join(', ')}` : ' through an unavailable route'}`;
    }
  }
}

export const splitReferences = (value: string): string[] => value.split(',').map((part) => part.trim()).filter(Boolean);
