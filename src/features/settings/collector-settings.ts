/**
 * The collector toggles the Settings screen renders. `promise` is the full
 * sentence the product commits to — `<label> — <what it watches>` — rendered
 * as one text node so the two strings PRODUCT.md fixes verbatim are on screen
 * byte for byte. The dev harness reuses this list rather than re-typing it.
 *
 * The backend owns which collectors exist and whether each is enabled; this
 * file owns what each one is called and what it promises to watch. Copy in
 * Rust would mean shipping the product's voice in a binary and translating it
 * through IPC for no reason.
 */

export interface CollectorSetting {
  id: string;
  label: string;
  promise: string;
  enabled: boolean;
}

/** Label and promise for every collector the backend can report. */
const COPY: Record<string, { label: string; promise: string }> = {
  process: {
    label: 'Programs',
    promise: 'Programs — applications and background processes starting and stopping',
  },
  listeners: {
    label: 'Ports',
    promise: 'Ports — which program is listening on which port, and when that changes',
  },
  files: {
    label: 'Watched folders',
    promise:
      'Watched folders — configuration files changing inside folders you choose. Values are never stored.',
  },
  services: {
    label: 'Windows services',
    promise: 'Windows services — services starting, stopping, failing and changing start type',
  },
  applications: {
    label: 'Installed programs',
    promise: 'Installed programs — installs, uninstalls and version changes',
  },
  updates: {
    label: 'Windows Update',
    promise: 'Windows Update — updates installing, and which ones',
  },
  drivers: {
    label: 'Drivers',
    promise: 'Drivers — device drivers being installed, updated or rolled back',
  },
  network: {
    label: 'Network',
    promise: 'Network — adapters, DNS servers, IP addresses and proxy settings changing',
  },
  startup: {
    label: 'Startup items',
    promise: 'Startup items — programs and scheduled tasks that run when Windows starts',
  },
  eventlog: {
    label: 'Windows event log',
    promise: 'Windows event log — application crashes and service failures the system recorded',
  },
  docker: {
    label: 'Docker',
    promise: 'Docker — containers, images and volumes starting, stopping and being rebuilt',
  },
  dev_env: {
    label: 'Developer environment',
    promise:
      'Developer environment — environment variables, PATH, installed tool versions, and the branch of folders you watch. Values are never stored.',
  },
};

/**
 * Copy for a collector id. An unknown id gets its own id back rather than
 * being hidden: a collector the backend reports and this table does not know
 * about is a gap to see, not one to paper over.
 */
export function collectorCopy(id: string): { label: string; promise: string } {
  return COPY[id] ?? { label: id, promise: `${id} — no description yet` };
}

/**
 * The fixture list, for the dev harness and for a preview outside the app.
 * The two opt-in collectors, in the order Settings shows them.
 */
export const defaultCollectorSettings: readonly CollectorSetting[] = [
  { id: 'docker', ...collectorCopy('docker'), enabled: false },
  { id: 'dev_env', ...collectorCopy('dev_env'), enabled: false },
];
