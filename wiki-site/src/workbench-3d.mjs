import * as THREE from 'three';
import { booking } from './demo-data.mjs';

/** Demand rendering: no animation loop, network requests, or hidden background work. */
export function createWorkbench(container, onSelect) {
  const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 1.5));
  container.append(renderer.domElement);
  const scene = new THREE.Scene(), camera = new THREE.OrthographicCamera(-7, 7, 5, -5, .1, 100);
  const target = new THREE.Vector3(0, 0, 0);
  camera.position.set(0, 5, 14); camera.lookAt(target);
  const ray = new THREE.Raycaster(), pointer = new THREE.Vector2(), objects = [], lines = [];
  let disposed = false, selected = 'intent', separated = true, visibleLayer = 'all', result = 'passed';
  const css = getComputedStyle(container);
  const muted = css.getPropertyValue('--line-strong').trim();
  const accent = css.getPropertyValue('--accent').trim();
  const render = () => { if (!disposed && !document.hidden) renderer.render(scene, camera); };
  for (const node of booking.nodes) {
    const mesh = new THREE.Mesh(new THREE.BoxGeometry(3.7, .12, 1.45), new THREE.MeshBasicMaterial({ color: node.layer === 2 ? '#ddd9d0' : '#eeece5' }));
    mesh.userData.id = node.id;
    scene.add(mesh); objects.push(mesh);
    const label = document.createElement('button');
    label.type = 'button'; label.className = 'three-label'; label.textContent = node.label;
    label.addEventListener('click', () => onSelect(node.id));
    container.append(label); mesh.userData.label = label;
  }
  const layout = () => {
    objects.forEach((mesh, index) => {
      const node = booking.nodes[index], column = index % 2;
      mesh.position.set(column ? 3.5 : -3.5, (1 - node.layer) * (separated ? 3.5 : 1.4), 0);
      mesh.visible = visibleLayer === 'all' || String(node.layer) === visibleLayer;
    });
    for (const line of lines) { scene.remove(line); line.geometry.dispose(); line.material.dispose(); }
    lines.length = 0;
    for (const edge of booking.edges) {
      const from = objects.find((item) => item.userData.id === edge.from), to = objects.find((item) => item.userData.id === edge.to);
      const line = new THREE.Line(new THREE.BufferGeometry().setFromPoints([from.position, to.position]), new THREE.LineBasicMaterial({ color: muted }));
      line.visible = from.visible && to.visible; scene.add(line); lines.push(line);
    }
    positionLabels(); render();
  };
  const positionLabels = () => {
    objects.forEach((mesh) => {
      const point = mesh.position.clone().project(camera), label = mesh.userData.label;
      label.hidden = !mesh.visible;
      label.style.left = `${(point.x + 1) / 2 * container.clientWidth}px`;
      label.style.top = `${(-point.y + 1) / 2 * container.clientHeight}px`;
    });
  };
  const resize = () => {
    const width = container.clientWidth, height = container.clientHeight;
    const aspect = width / height, halfHeight = Math.max(5, 6 / aspect);
    renderer.setSize(width, height); camera.left = -halfHeight * aspect; camera.right = halfHeight * aspect;
    camera.top = halfHeight; camera.bottom = -halfHeight; camera.updateProjectionMatrix(); positionLabels(); render();
  };
  const select = (id) => {
    selected = id;
    objects.forEach((mesh) => {
      const node = booking.nodes.find((item) => item.id === mesh.userData.id);
      mesh.material.color.set(mesh.userData.id === id ? accent : node.layer === 2 && result === 'passed' ? '#dce6d7' : '#e6e1d7');
      mesh.userData.label.dataset.selected = String(mesh.userData.id === id);
    }); render();
  };
  const click = (event) => {
    const rect = renderer.domElement.getBoundingClientRect();
    pointer.set((event.clientX - rect.left) / rect.width * 2 - 1, -(event.clientY - rect.top) / rect.height * 2 + 1);
    ray.setFromCamera(pointer, camera);
    const hit = ray.intersectObjects(objects.filter((item) => item.visible))[0];
    if (hit) onSelect(hit.object.userData.id);
  };
  const contextLost = (event) => { event.preventDefault(); dispose(); container.textContent = 'WebGL context lost. Use the complete evidence cards above.'; };
  renderer.domElement.addEventListener('click', click);
  renderer.domElement.addEventListener('webglcontextlost', contextLost);
  const observer = new ResizeObserver(resize); observer.observe(container);
  const visibility = () => render(); document.addEventListener('visibilitychange', visibility);
  function dispose() {
    if (disposed) return; disposed = true; observer.disconnect(); document.removeEventListener('visibilitychange', visibility);
    renderer.domElement.removeEventListener('click', click); renderer.domElement.removeEventListener('webglcontextlost', contextLost);
    for (const object of [...objects, ...lines]) { object.geometry.dispose(); object.material.dispose(); object.userData.label?.remove(); }
    scene.clear(); renderer.dispose(); renderer.forceContextLoss(); renderer.domElement.remove();
  }
  layout(); resize();
  return {
    select, dispose, layer(value) { visibleLayer = value; layout(); },
    zoom(value) { camera.zoom = value; camera.updateProjectionMatrix(); positionLabels(); render(); },
    explode(value) { separated = value; layout(); },
    reset() { camera.zoom = 1; camera.lookAt(target); camera.updateProjectionMatrix(); positionLabels(); render(); },
    fit() { camera.lookAt(objects.find((item) => item.userData.id === selected)?.position || target); camera.updateProjectionMatrix(); positionLabels(); render(); },
    setResult(value) {
      result = value;
      for (const mesh of objects) {
        const node = booking.nodes.find((item) => item.id === mesh.userData.id);
        mesh.userData.label.textContent = `${node.label}${node.layer === 2 ? ` · ${value}` : ''}`;
      }
      select(selected); positionLabels(); container.dataset.verdict = value;
      container.setAttribute('aria-label', `Evidence model: ${value}. Build labels and exact evidence are in the cards above.`);
    },
  };
}
