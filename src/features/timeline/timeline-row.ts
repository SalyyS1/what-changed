/**
 * One line of the timeline as the screen renders it. Fixed now so the list,
 * its fixtures and the query layer that will feed it agree on a shape.
 */
export interface TimelineRow {
  id: string;
  time: string;
  title: string;
  before?: string;
  after?: string;
}
