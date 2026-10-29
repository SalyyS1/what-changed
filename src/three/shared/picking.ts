import { Camera, InstancedMesh, Raycaster, Vector2 } from 'three';

export interface PointerNdc { readonly x: number; readonly y: number }

export interface PickingCallbacks<TNode> {
  hover(index: number | null, node: TNode | null, clientX: number, clientY: number): void;
  select(index: number, node: TNode): void;
}

export interface PickingController { dispose(): void }

export interface ChainMember { readonly chain_ids: readonly string[] }

export class PointerClickGuard {
  #x = 0;
  #y = 0;
  #pressed = false;
  #moved = false;

  press(x: number, y: number): void {
    this.#x = x;
    this.#y = y;
    this.#pressed = true;
    this.#moved = false;
  }

  move(x: number, y: number): void {
    if (this.#pressed && Math.hypot(x - this.#x, y - this.#y) > 4) this.#moved = true;
  }

  release(): boolean {
    const isClick = this.#pressed && !this.#moved;
    this.#pressed = false;
    this.#moved = false;
    return isClick;
  }

  cancel(): void { this.#pressed = false; this.#moved = false; }
}

export function pointerToNdc(clientX: number, clientY: number, rect: DOMRect): PointerNdc {
  return {
    x: ((clientX - rect.left) / Math.max(1, rect.width)) * 2 - 1,
    y: -((clientY - rect.top) / Math.max(1, rect.height)) * 2 + 1,
  };
}

export function chainMembership(node: ChainMember | null): ReadonlySet<string> | null {
  return node && node.chain_ids.length > 0 ? new Set(node.chain_ids) : null;
}

export function createPickingController<TNode>(
  element: HTMLElement,
  camera: Camera,
  mesh: InstancedMesh,
  nodes: readonly TNode[],
  callbacks: PickingCallbacks<TNode>,
  now: () => number = performance.now.bind(performance),
): PickingController {
  const raycaster = new Raycaster();
  const ndc = new Vector2();
  let lastPick = Number.NEGATIVE_INFINITY;
  let hovered: number | null = null;
  let inside = false;
  const clickGuard = new PointerClickGuard();

  function pick(event: PointerEvent, force = false): number | null {
    if (!inside) return null;
    const time = now();
    if (!force && time - lastPick < 1000 / 30) return hovered;
    lastPick = time;
    const point = pointerToNdc(event.clientX, event.clientY, element.getBoundingClientRect());
    ndc.set(point.x, point.y);
    raycaster.setFromCamera(ndc, camera);
    const instanceId = raycaster.intersectObject(mesh, false)[0]?.instanceId ?? null;
    if (instanceId !== hovered || force) {
      hovered = instanceId;
      callbacks.hover(instanceId, instanceId === null ? null : nodes[instanceId] ?? null, event.clientX, event.clientY);
    }
    return hovered;
  }

  function enter(event: PointerEvent): void { inside = true; pick(event, true); }
  function down(event: PointerEvent): void {
    if (event.button === 0 && inside) clickGuard.press(event.clientX, event.clientY);
  }
  function move(event: PointerEvent): void { clickGuard.move(event.clientX, event.clientY); pick(event); }
  function leave(event: PointerEvent): void {
    inside = false;
    clickGuard.cancel();
    hovered = null;
    callbacks.hover(null, null, event.clientX, event.clientY);
  }
  function up(event: PointerEvent): void {
    if (!clickGuard.release()) return;
    const index = pick(event, true);
    const node = index === null ? undefined : nodes[index];
    if (index !== null && node) callbacks.select(index, node);
  }
  const cancel = (): void => clickGuard.cancel();
  element.addEventListener('pointerenter', enter);
  element.addEventListener('pointerdown', down);
  element.addEventListener('pointermove', move);
  element.addEventListener('pointerup', up);
  element.addEventListener('pointercancel', cancel);
  element.addEventListener('pointerleave', leave);

  return { dispose(): void {
    element.removeEventListener('pointerenter', enter);
    element.removeEventListener('pointerdown', down);
    element.removeEventListener('pointermove', move);
    element.removeEventListener('pointerup', up);
    element.removeEventListener('pointercancel', cancel);
    element.removeEventListener('pointerleave', leave);
  } };
}
