import * as THREE from 'three';
import { RoundedBoxGeometry } from 'three/addons/geometries/RoundedBoxGeometry.js';
import { booking, bookingNodeCopy } from './demo-data.mjs';

/** The DOM buttons and the scene share the same explicit evidence graph. */
export function createWorkbench(container, onSelect, { language = 'en' } = {}) {
  const vi = language === 'vi';
  const resultLabel = (value) => vi ? ({ passed: 'đạt', failed: 'lỗi', blocked: 'bị chặn', stale: 'đã cũ' }[value] ?? value) : value;
  const canvas = document.createElement('canvas');
  let renderer;
  try {
    renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true });
  } catch (cause) {
    canvas.remove();
    throw new Error('3D is unavailable. The complete evidence cards remain available.', { cause });
  }
  renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 1.5));
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 1.15;
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFShadowMap;
  canvas.setAttribute('aria-hidden', 'true');
  canvas.style.width = '100%'; canvas.style.height = '100%';
  container.append(canvas);

  const scene = new THREE.Scene();
  const camera = new THREE.OrthographicCamera(-8, 8, 6, -6, .1, 100);
  const ray = new THREE.Raycaster(), pointer = new THREE.Vector2();
  const resources = new Set(), records = [], platforms = [], links = [];
  const css = getComputedStyle(container);
  const color = (name, fallback) => new THREE.Color(css.getPropertyValue(name).trim() || fallback);
  const ink = color('--ink', '#252923'), paper = color('--paper', '#f7f5ef');
  const accent = color('--accent', '#42604b'), stroke = color('--line-strong', '#b7b7ac');
  const target = new THREE.Vector3(), homeAngle = Math.atan2(9, 16);
  let selected = 'intent', separated = true, visibleLayer = 'all', result = 'passed';
  let angle = homeAngle, disposed = false, frame = 0, observer;
  let width = 1, height = 1;
  const own = (resource) => { resources.add(resource); return resource; };
  const matte = (value) => own(new THREE.MeshStandardMaterial({ color: value, roughness: .86, metalness: .02 }));
  const render = () => {
    if (disposed || document.hidden || frame) return;
    frame = window.requestAnimationFrame(() => {
      frame = 0;
      if (!disposed && !document.hidden) renderer.render(scene, camera);
    });
  };

  scene.add(new THREE.HemisphereLight('#ffffff', paper.clone().lerp(ink, .24), 2.2));
  const key = new THREE.DirectionalLight('#ffffff', 3.1);
  key.position.set(-6, 12, 8); key.castShadow = true;
  key.shadow.mapSize.set(1024, 1024);
  Object.assign(key.shadow.camera, { left: -11, right: 11, top: 12, bottom: -12, near: 1, far: 40 });
  key.shadow.camera.updateProjectionMatrix();
  key.shadow.bias = -.0005; key.shadow.normalBias = .035; key.shadow.radius = 3;
  scene.add(key);
  const fill = new THREE.DirectionalLight('#ffffff', .65);
  fill.position.set(8, 4, -5); scene.add(fill);

  const platformGeometry = own(new RoundedBoxGeometry(10.6, .26, 3.5, 3, .11));
  const tileGeometry = own(new RoundedBoxGeometry(4.28, .5, 1.96, 3, .1));
  const railGeometry = own(new THREE.BoxGeometry(3.65, .035, .055));
  for (const layer of [...new Set(booking.nodes.map((node) => node.layer))]) {
    const group = new THREE.Group(); group.userData.layer = layer;
    const platform = new THREE.Mesh(platformGeometry, matte(paper.clone().lerp(ink, .035 + layer * .025)));
    platform.castShadow = true; platform.receiveShadow = true;
    group.add(platform); scene.add(group); platforms.push(group);
  }
  for (const [index, node] of booking.nodes.entries()) {
    const group = platforms.find((item) => item.userData.layer === node.layer);
    const mesh = new THREE.Mesh(tileGeometry, matte(paper.clone().lerp(new THREE.Color('#ffffff'), .34)));
    mesh.position.set(index % 2 ? 2.42 : -2.42, .47, 0);
    mesh.castShadow = true; mesh.receiveShadow = true; mesh.userData.id = node.id;
    const rail = new THREE.Mesh(railGeometry, matte(stroke));
    rail.position.set(0, .27, -.68); mesh.add(rail); group.add(mesh);
    const label = document.createElement('button');
    const kind = document.createElement('span'), title = document.createElement('strong');
    label.type = 'button'; label.className = 'three-label'; label.dataset.node = node.id;
    label.style.boxSizing = 'border-box'; label.style.textAlign = 'left';
    title.textContent = bookingNodeCopy(node, language).label; label.append(kind, title); container.append(label);
    const activate = () => onSelect(node.id);
    label.addEventListener('click', activate);
    records.push({ node, mesh, rail, label, kind, activate, column: index % 2 });
  }
  for (const edge of booking.edges) {
    const material = own(new THREE.LineBasicMaterial({ color: stroke, transparent: true, opacity: .72 }));
    const line = new THREE.Line(own(new THREE.BufferGeometry()), material);
    line.userData.edge = edge; scene.add(line); links.push({ edge, line });
  }

  const worldPosition = (record) => record.mesh.getWorldPosition(new THREE.Vector3());
  const updateCamera = () => {
    camera.position.set(Math.sin(angle) * 19, 11, Math.cos(angle) * 19).add(target);
    camera.lookAt(target); camera.updateMatrixWorld(true);
  };
  // Frame the solid platforms, leaving enough room for the readable HTML labels.
  const frameModel = () => {
    scene.updateMatrixWorld(true);
    const min = new THREE.Vector3(Infinity, Infinity, Infinity), max = min.clone().negate();
    for (const group of platforms) if (group.visible) {
      const bounds = new THREE.Box3().setFromObject(group);
      for (const x of [bounds.min.x, bounds.max.x]) for (const y of [bounds.min.y, bounds.max.y]) for (const z of [bounds.min.z, bounds.max.z]) {
        const point = new THREE.Vector3(x, y, z).applyMatrix4(camera.matrixWorldInverse);
        min.min(point); max.max(point);
      }
    }
    if (!Number.isFinite(min.x)) return;
    const aspect = width / height, centerX = (min.x + max.x) / 2, centerY = (min.y + max.y) / 2;
    const halfHeight = Math.max((max.y - min.y) / 2 + .8, ((max.x - min.x) / 2 + .6) / aspect);
    camera.left = centerX - halfHeight * aspect; camera.right = centerX + halfHeight * aspect;
    camera.top = centerY + halfHeight; camera.bottom = centerY - halfHeight;
    camera.updateProjectionMatrix();
  };
  const positionLabels = () => {
    if (disposed) return;
    const padding = width < 460 ? 9 : 16, bottomPadding = 56, gap = 10;
    const labelWidth = Math.min(174, (width - padding * 2 - gap) / 2);
    const columns = [[], []];
    for (const record of records) {
      const { mesh, label, column } = record;
      label.hidden = !mesh.parent.visible;
      if (label.hidden) continue;
      label.style.width = `${Math.max(96, labelWidth)}px`;
      label.style.maxWidth = `${Math.max(96, labelWidth)}px`;
      const point = worldPosition(record).add(new THREE.Vector3(0, .31, .05)).project(camera);
      const idealX = (point.x + 1) / 2 * width, idealY = (-point.y + 1) / 2 * height;
      const half = label.getBoundingClientRect().height / 2;
      // Separate the two label lanes without changing any evidence relationship.
      const x = column ? Math.max(width / 2 + gap / 2 + labelWidth / 2, idealX) : Math.min(width / 2 - gap / 2 - labelWidth / 2, idealX);
      label.style.left = `${THREE.MathUtils.clamp(x, padding + labelWidth / 2, width - padding - labelWidth / 2)}px`;
      columns[column].push({ label, idealY, half });
    }
    for (const column of columns) {
      column.sort((a, b) => a.idealY - b.idealY);
      let bottom = padding;
      for (const item of column) { item.y = Math.max(item.idealY, bottom + item.half); bottom = item.y + item.half + gap; }
      let top = height - bottomPadding;
      for (const item of [...column].reverse()) {
        item.y = Math.min(item.y, top - item.half);
        top = item.y - item.half - gap;
      }
      for (const item of column) item.label.style.top = `${Math.max(padding + item.half, item.y)}px`;
    }
  };
  const refresh = () => { positionLabels(); render(); };
  const updateLinks = () => {
    scene.updateMatrixWorld(true);
    for (const { edge, line } of links) {
      const from = records.find((record) => record.node.id === edge.from);
      const to = records.find((record) => record.node.id === edge.to);
      const start = worldPosition(from), end = worldPosition(to);
      start.y += .03; end.y += .03;
      start.z += 1.01; end.z += from.node.layer === to.node.layer ? 1.01 : -1.01;
      const middleA = start.clone(), middleB = end.clone();
      if (from.node.layer === to.node.layer) { middleA.z += .4; middleB.z += .4; }
      else { middleA.y -= .5; middleB.y += .5; }
      line.geometry.setFromPoints([start, middleA, middleB, end]);
      line.visible = from.mesh.parent.visible && to.mesh.parent.visible;
    }
  };
  const layout = () => {
    for (const group of platforms) {
      const layer = group.userData.layer;
      group.position.set(0, (1 - layer) * (separated ? 2.5 : 1.05), (layer - 1) * (separated ? 1.7 : 2.45));
      group.visible = visibleLayer === 'all' || String(layer) === visibleLayer;
    }
    updateLinks(); updateCamera(); frameModel(); refresh();
  };
  const select = (id) => {
    if (disposed || !records.some((record) => record.node.id === id)) return;
    selected = id;
    for (const { node, mesh, rail, label, kind } of records) {
      const active = node.id === id;
      mesh.material.color.copy(paper).lerp(active ? ink : new THREE.Color('#ffffff'), active ? .15 : .34);
      // A result can pass. Selection never assigns an owner verdict to a rule.
      const statusColor = node.layer === 2 && result === 'passed' ? accent : stroke;
      rail.material.color.copy(active ? ink : statusColor);
      label.dataset.selected = String(active); label.setAttribute('aria-pressed', String(active));
      label.dataset.verdict = node.layer === 2 ? result : '';
      const copy = bookingNodeCopy(node, language);
      kind.textContent = `${copy.kind}${node.layer === 2 ? ` · ${resultLabel(result)}` : ''}`;
      label.setAttribute('aria-label', `${copy.kind}: ${copy.label}${node.layer === 2 ? `, ${resultLabel(result)}` : ''}`);
    }
    for (const { edge, line } of links) {
      const active = edge.from === id || edge.to === id;
      line.material.color.copy(active ? ink : stroke); line.material.opacity = active ? .85 : .5;
    }
    refresh();
  };
  const resize = () => {
    if (disposed || !container.clientWidth || !container.clientHeight) return;
    width = container.clientWidth; height = container.clientHeight;
    renderer.setSize(width, height, false); frameModel(); refresh();
  };
  const click = (event) => {
    if (disposed) return;
    const rect = canvas.getBoundingClientRect();
    if (!rect.width || !rect.height) return;
    pointer.set((event.clientX - rect.left) / rect.width * 2 - 1, -(event.clientY - rect.top) / rect.height * 2 + 1);
    ray.setFromCamera(pointer, camera);
    const hit = ray.intersectObjects(records.filter((record) => record.mesh.parent.visible).map((record) => record.mesh), false)[0];
    if (hit) onSelect(hit.object.userData.id);
  };
  const visibility = () => {
    if (document.hidden && frame) { window.cancelAnimationFrame(frame); frame = 0; }
    else if (!document.hidden) resize();
  };
  const contextLost = (event) => {
    if (disposed) return;
    event.preventDefault(); dispose(true);
    const fallback = document.createElement('p');
    fallback.className = 'three-fallback'; fallback.setAttribute('role', 'status');
    fallback.textContent = vi ? 'Đồ họa 3D không khả dụng. Bạn vẫn xem được đầy đủ sơ đồ và các mối liên kết.' : '3D graphics became unavailable. The complete evidence cards and connection list remain available.';
    container.append(fallback);
    container.dispatchEvent(new CustomEvent('workbenchfallback', { bubbles: true, detail: { reason: 'context-lost' } }));
  };
  function dispose(contextIsLost = false) {
    if (disposed) return;
    disposed = true;
    if (frame) window.cancelAnimationFrame(frame);
    observer?.disconnect(); document.removeEventListener('visibilitychange', visibility);
    window.removeEventListener('pagehide', dispose);
    canvas.removeEventListener('click', click); canvas.removeEventListener('webglcontextlost', contextLost);
    for (const { label, activate } of records) { label.removeEventListener('click', activate); label.remove(); }
    for (const resource of resources) resource.dispose();
    key.shadow.dispose(); scene.clear(); renderer.dispose();
    if (contextIsLost !== true) renderer.forceContextLoss();
    canvas.remove();
  }
  try {
    canvas.addEventListener('click', click); canvas.addEventListener('webglcontextlost', contextLost);
    document.addEventListener('visibilitychange', visibility); window.addEventListener('pagehide', dispose, { once: true });
    observer = new ResizeObserver(resize); observer.observe(container);
    layout(); resize(); select(selected);
  } catch (error) { dispose(); throw error; }

  return {
    select, dispose,
    layer(value) { if (disposed) return; visibleLayer = String(value); layout(); },
    zoom(value) {
      if (disposed || !Number.isFinite(value)) return;
      camera.zoom = THREE.MathUtils.clamp(value, .65, 1.8); camera.updateProjectionMatrix(); refresh();
    },
    explode(value) { if (disposed) return; separated = Boolean(value); layout(); },
    rotate(direction = 1) {
      if (disposed) return;
      const sign = direction === 'left' || Number(direction) < 0 ? -1 : 1;
      angle += sign * Math.PI / 12;
      if (angle > .98) angle = -.45;
      if (angle < -.65) angle = .85;
      updateCamera(); frameModel(); refresh();
    },
    reset() {
      if (disposed) return;
      angle = homeAngle; target.set(0, 0, 0); camera.zoom = 1;
      updateCamera(); frameModel(); refresh();
    },
    fit() {
      if (disposed) return;
      target.copy(worldPosition(records.find((record) => record.node.id === selected)));
      camera.zoom = 1.15; updateCamera(); frameModel();
      const halfWidth = (camera.right - camera.left) / 2, halfHeight = (camera.top - camera.bottom) / 2;
      camera.left = -halfWidth; camera.right = halfWidth; camera.top = halfHeight; camera.bottom = -halfHeight;
      camera.updateProjectionMatrix(); refresh();
    },
    setResult(value) {
      if (disposed) return;
      result = String(value); container.dataset.verdict = result;
      container.setAttribute('aria-label', vi ? `Mô hình bằng chứng: ${resultLabel(result)}. Xem nhãn phiên bản và bằng chứng cụ thể trong các thẻ.` : `Evidence model: ${result}. Build labels and exact evidence are available in the evidence cards.`);
      select(selected);
    },
  };
}
