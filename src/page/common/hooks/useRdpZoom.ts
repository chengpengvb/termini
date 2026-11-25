import { type RefObject, useEffect, useRef } from 'react';
import type { RdpFocus } from '@/common/model/req/event/WsRdp.ts';
import { mobileZoomMax } from '@/page/common/hooks/useRdpLayout.ts';
import { resolveRdpContentBox } from '@/page/common/util/rdpUtil.ts';
import type { CursorPoint } from '@/page/common/hooks/useRdpPointer.ts';

const ZOOM_MIN = 1;
const EDGE_SNAP_PX = 14;
const ZOOM_SNAP_RATIO = 0.06;
const FLING_DECAY_SECONDS = 0.18;
const SPRING_FREQUENCY = 22;
type ViewMode = 'strict' | 'elastic' | 'animation';

interface MotionState {
  kind: 'fling' | 'spring';
  value: ViewState;
  velocity: ViewState;
  target: ViewState;
  previous: number;
  started: number;
}

interface PanState {
  value: ViewState;
  vx: number;
  vy: number;
  time: number;
}

interface ViewState {
  tx: number;
  ty: number;
  zoom: number;
}

interface PinchState {
  anchorX: number;
  anchorY: number;
  distance: number;
  zoom: number;
}

interface VisibleRect {
  left: number;
  top: number;
  right: number;
  bottom: number;
  scale: number;
}

interface HostBox {
  padLeft: number;
  padTop: number;
  viewWidth: number;
  viewHeight: number;
}

function identityView(): ViewState {
  return { tx: 0, ty: 0, zoom: ZOOM_MIN };
}

function readHostBox(host: HTMLElement): HostBox {
  const style = getComputedStyle(host);
  const padLeft = parseFloat(style.paddingLeft);
  const padTop = parseFloat(style.paddingTop);
  return {
    padLeft,
    padTop,
    viewWidth: host.clientWidth - padLeft - parseFloat(style.paddingRight),
    viewHeight: host.clientHeight - padTop - parseFloat(style.paddingBottom),
  };
}

function twoPoints(points: Iterable<CursorPoint>): [CursorPoint, CursorPoint] | null {
  const [first, second] = [...points];
  if (!first || !second) {
    return null;
  }
  return [first, second];
}

function rubber(distance: number, limit: number): number {
  return Math.sign(distance) * limit * (1 - Math.exp(-Math.abs(distance) * 0.45 / limit));
}

function clampAxis(offset: number, scaledSize: number, viewStart: number, viewSize: number, next: number, mode: ViewMode): number {
  // Let any desktop edge reach the middle of the usable viewport. This leaves
  // room around bottom inputs without letting the desktop disappear entirely.
  const overlap = Math.min(scaledSize, viewSize / 2);
  const min = viewStart + overlap - scaledSize - offset;
  const max = viewStart + viewSize - overlap - offset;
  const bounded = Math.max(min, Math.min(max, next));
  const limit = Math.min(64, viewSize * 0.18, scaledSize * 0.35);
  if (mode === 'animation') {
    return bounded + Math.max(-limit, Math.min(limit, next - bounded));
  }
  return mode === 'elastic' ? bounded + rubber(next - bounded, limit) : bounded;
}

function readCanvasBox(canvas: HTMLCanvasElement) {
  const scale = Math.min(canvas.offsetWidth / canvas.width, canvas.offsetHeight / canvas.height);
  const width = canvas.width * scale;
  const height = canvas.height * scale;
  return {width, height, insetLeft: (canvas.offsetWidth - width) / 2, insetTop: (canvas.offsetHeight - height) / 2};
}

function minimumZoom(canvas: HTMLCanvasElement, box: HostBox): number {
  const content = readCanvasBox(canvas);
  if (!content.width || !content.height || box.viewWidth <= 0 || box.viewHeight <= 0) {
    return ZOOM_MIN;
  }
  return Math.min(ZOOM_MIN, box.viewWidth / content.width, box.viewHeight / content.height);
}

function clampView(canvas: HTMLCanvasElement, host: HTMLElement, next: ViewState, mode: ViewMode = 'strict'): ViewState {
  const box = readHostBox(host);
  const min = minimumZoom(canvas, box);
  const max = mobileZoomMax();
  const boundedZoom = Math.max(min, Math.min(max, next.zoom));
  const zoom = mode === 'strict' ? boundedZoom : next.zoom;
  const content = readCanvasBox(canvas);
  return {
    tx: clampAxis(canvas.offsetLeft + content.insetLeft * zoom, content.width * zoom, box.padLeft, box.viewWidth, next.tx, mode),
    ty: clampAxis(canvas.offsetTop + content.insetTop * zoom, content.height * zoom, box.padTop, box.viewHeight, next.ty, mode),
    zoom,
  };
}

function sameFocus(a: RdpFocus | null, b: RdpFocus | null): boolean {
  if (a === null || b === null) {
    return a === b;
  }
  return a.left === b.left && a.top === b.top && a.right === b.right && a.bottom === b.bottom;
}

export function useRdpZoom(canvasRef: RefObject<HTMLCanvasElement | null>, onViewChange: () => void, onFocusChange: (focus: RdpFocus | null) => void) {
  const viewRef = useRef<ViewState>(identityView());
  const pinchRef = useRef<PinchState | null>(null);
  const focusRef = useRef<RdpFocus | null>(null);
  const centerRef = useRef<CursorPoint | null>(null);
  const panRef = useRef<PanState | null>(null);
  const pinchCenterRef = useRef<CursorPoint | null>(null);
  const motionRef = useRef<MotionState | null>(null);
  const frameRef = useRef(0);

  const resolveCanvas = (): HTMLCanvasElement | null => {
    const canvas = canvasRef.current;
    if (!canvas || canvas.width === 0 || canvas.height === 0) {
      return null;
    }
    return canvas;
  };

  const canvasPoint = (clientX: number, clientY: number): CursorPoint | null => {
    const canvas = resolveCanvas();
    const box = canvas ? resolveRdpContentBox(canvas) : null;
    if (!box) {
      return null;
    }
    return { x: (clientX - box.left) / box.scale, y: (clientY - box.top) / box.scale };
  };

  const publishFocus = (next: RdpFocus | null): void => {
    if (sameFocus(focusRef.current, next)) {
      return;
    }
    focusRef.current = next;
    onFocusChange(next);
  };

  const emitFocus = (): void => {
    const canvas = resolveCanvas();
    const host = canvas?.parentElement;
    if (!canvas || !host) {
      return;
    }
    const box = readHostBox(host);
    const rect = host.getBoundingClientRect();
    const start = canvasPoint(rect.left + box.padLeft, rect.top + box.padTop);
    const end = canvasPoint(rect.left + box.padLeft + box.viewWidth, rect.top + box.padTop + box.viewHeight);
    if (!start || !end) {
      return;
    }
    const focus: RdpFocus = {
      left: Math.floor(Math.max(0, Math.min(canvas.width, start.x))),
      top: Math.floor(Math.max(0, Math.min(canvas.height, start.y))),
      right: Math.ceil(Math.max(0, Math.min(canvas.width, end.x))),
      bottom: Math.ceil(Math.max(0, Math.min(canvas.height, end.y))),
    };
    if (focus.right <= focus.left || focus.bottom <= focus.top) {
      publishFocus(null);
      return;
    }
    publishFocus(focus.left === 0 && focus.top === 0 && focus.right === canvas.width && focus.bottom === canvas.height ? null : focus);
  };

  const applyView = (next: ViewState, mode: ViewMode = 'strict', focus = true): void => {
    const canvas = resolveCanvas();
    const host = canvas?.parentElement;
    if (!canvas || !host) {
      return;
    }
    const box = readHostBox(host);
    if (!canvas.offsetWidth || !canvas.offsetHeight || box.viewWidth <= 0 || box.viewHeight <= 0) {
      return;
    }
    const view = clampView(canvas, host, next, mode);
    viewRef.current = view;
    const content = readCanvasBox(canvas);
    centerRef.current = {
      x: ((box.padLeft + box.viewWidth / 2 - canvas.offsetLeft - view.tx) / view.zoom - content.insetLeft) / content.width,
      y: ((box.padTop + box.viewHeight / 2 - canvas.offsetTop - view.ty) / view.zoom - content.insetTop) / content.height,
    };
    canvas.style.transformOrigin = '0 0';
    canvas.style.transform = `translate(${view.tx}px, ${view.ty}px) scale(${view.zoom})`;
    onViewChange();
    if (focus) {
      emitFocus();
    }
  };

  const cancelMotion = (settle = true): boolean => {
    const moving = motionRef.current !== null;
    cancelAnimationFrame(frameRef.current);
    frameRef.current = 0;
    motionRef.current = null;
    panRef.current = null;
    if (settle) {
      applyView(viewRef.current);
    }
    return moving;
  };

  const rawView = (value: ViewState): ViewState => {
    const canvas = resolveCanvas();
    const host = canvas?.parentElement;
    if (!canvas || !host) {
      return {...value};
    }
    const box = readHostBox(host);
    const content = readCanvasBox(canvas);
    const restore = (position: number, offset: number, size: number, start: number, length: number): number => {
      const bounded = clampAxis(offset, size, start, length, position, 'strict');
      const limit = Math.min(64, length * 0.18, size * 0.35);
      const distance = position - bounded;
      const fraction = Math.min(0.99, Math.abs(distance) / limit);
      return bounded - Math.sign(distance) * Math.log(1 - fraction) * limit / 0.45;
    };
    return {
      tx: restore(value.tx, canvas.offsetLeft + content.insetLeft * value.zoom, content.width * value.zoom, box.padLeft, box.viewWidth),
      ty: restore(value.ty, canvas.offsetTop + content.insetTop * value.zoom, content.height * value.zoom, box.padTop, box.viewHeight),
      zoom: value.zoom,
    };
  };

  const reducedMotion = (): boolean => window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  const snapAxis = (offset: number, size: number, start: number, length: number, value: number): number => {
    const first = start - offset;
    const last = start + length - size - offset;
    const candidates = size <= length + 1 ? [first, last, (first + last) / 2] : [first, last];
    let result = value;
    let distance = Math.min(EDGE_SNAP_PX, length * 0.06);
    for (const candidate of candidates) {
      if (Math.abs(candidate - value) < distance) {
        result = candidate;
        distance = Math.abs(candidate - value);
      }
    }
    return result;
  };

  const settleTarget = (value: ViewState, snapScale: boolean): ViewState => {
    const canvas = resolveCanvas();
    const host = canvas?.parentElement;
    if (!canvas || !host) {
      return value;
    }
    const box = readHostBox(host);
    const fitZoom = minimumZoom(canvas, box);
    const zoom = snapScale && Math.abs(value.zoom / fitZoom - 1) <= ZOOM_SNAP_RATIO ? fitZoom : Math.max(fitZoom, Math.min(mobileZoomMax(), value.zoom));
    const rect = host.getBoundingClientRect();
    const anchor = pinchCenterRef.current ?? {x: rect.left + box.padLeft + box.viewWidth / 2, y: rect.top + box.padTop + box.viewHeight / 2};
    const x = anchor.x - rect.left - canvas.offsetLeft;
    const y = anchor.y - rect.top - canvas.offsetTop;
    const factor = zoom / value.zoom;
    const target = clampView(canvas, host, {tx: x - (x - value.tx) * factor, ty: y - (y - value.ty) * factor, zoom});
    const content = readCanvasBox(canvas);
    return clampView(canvas, host, {
      tx: snapAxis(canvas.offsetLeft + content.insetLeft * zoom, content.width * zoom, box.padLeft, box.viewWidth, target.tx),
      ty: snapAxis(canvas.offsetTop + content.insetTop * zoom, content.height * zoom, box.padTop, box.viewHeight, target.ty),
      zoom,
    });
  };

  const animate = (stamp: number): void => {
    const motion = motionRef.current;
    const canvas = resolveCanvas();
    const host = canvas?.parentElement;
    if (!motion || !canvas || !host) {
      cancelMotion(false);
      return;
    }
    const dt = Math.min(0.032, Math.max(0, (stamp - motion.previous) / 1000));
    motion.previous = stamp;
    if (motion.kind === 'fling') {
      const decay = Math.exp(-dt / FLING_DECAY_SECONDS);
      motion.value.tx += motion.velocity.tx * FLING_DECAY_SECONDS * (1 - decay);
      motion.value.ty += motion.velocity.ty * FLING_DECAY_SECONDS * (1 - decay);
      motion.velocity.tx *= decay;
      motion.velocity.ty *= decay;
      const bounded = clampView(canvas, host, motion.value);
      if (Math.abs(bounded.tx - motion.value.tx) > 0.1 || Math.abs(bounded.ty - motion.value.ty) > 0.1 || Math.hypot(motion.velocity.tx, motion.velocity.ty) < 25 || stamp - motion.started > 1000) {
        motion.kind = 'spring';
        motion.target = settleTarget(motion.value, false);
      }
    } else {
      // Analytic critically damped spring: independent of refresh rate and
      // interruptible on the next pointer down. Velocities are CSS px / second.
      const decay = Math.exp(-SPRING_FREQUENCY * dt);
      for (const key of ['tx', 'ty', 'zoom'] as const) {
        const delta = motion.value[key] - motion.target[key];
        const c = motion.velocity[key] + SPRING_FREQUENCY * delta;
        motion.value[key] = motion.target[key] + (delta + c * dt) * decay;
        motion.velocity[key] = (motion.velocity[key] - SPRING_FREQUENCY * c * dt) * decay;
      }
      if ((Math.hypot(motion.value.tx - motion.target.tx, motion.value.ty - motion.target.ty) < 0.1 && Math.abs(motion.value.zoom - motion.target.zoom) < 0.0005 && Math.hypot(motion.velocity.tx, motion.velocity.ty) < 3) || stamp - motion.started > 1600) {
        motionRef.current = null;
        frameRef.current = 0;
        applyView(motion.target);
        return;
      }
    }
    applyView(motion.value, 'animation', false);
    frameRef.current = requestAnimationFrame(animate);
  };

  const finishGesture = (stamp?: number): void => {
    const pan = panRef.current;
    const value = {...viewRef.current};
    const canvas = resolveCanvas();
    const host = canvas?.parentElement;
    panRef.current = null;
    if (!canvas || !host) {
      return;
    }
    const bounded = clampView(canvas, host, value);
    const target = settleTarget(value, stamp === undefined);
    const vx = pan && stamp !== undefined && stamp - pan.time < 80 ? pan.vx : 0;
    const vy = pan && stamp !== undefined && stamp - pan.time < 80 ? pan.vy : 0;
    if (reducedMotion()) {
      applyView(target);
      return;
    }
    const fling = Math.hypot(vx, vy) >= 120 && Math.abs(value.tx - bounded.tx) < 0.1 && Math.abs(value.ty - bounded.ty) < 0.1;
    if (!fling && Math.hypot(target.tx - value.tx, target.ty - value.ty) < 0.1 && Math.abs(target.zoom - value.zoom) < 0.0005) {
      applyView(target);
      return;
    }
    const now = performance.now();
    motionRef.current = {kind: fling ? 'fling' : 'spring', value, velocity: {tx: vx, ty: vy, zoom: 0}, target, previous: now, started: now};
    frameRef.current = requestAnimationFrame(animate);
  };

  const beginPan = (stamp: number): boolean => {
    const interrupted = cancelMotion(false);
    pinchCenterRef.current = null;
    panRef.current = {value: rawView(viewRef.current), vx: 0, vy: 0, time: stamp};
    return interrupted;
  };

  const panBy = (dx: number, dy: number, stamp?: number): void => {
    const pan = panRef.current;
    if (pan && stamp !== undefined) {
      const elapsed = stamp - pan.time;
      const dt = Math.max(8, elapsed) / 1000;
      pan.vx = elapsed > 100 ? 0 : pan.vx * 0.55 + Math.max(-2000, Math.min(2000, dx / dt)) * 0.45;
      pan.vy = elapsed > 100 ? 0 : pan.vy * 0.55 + Math.max(-2000, Math.min(2000, dy / dt)) * 0.45;
      pan.time = stamp;
      pan.value.tx += dx;
      pan.value.ty += dy;
      applyView(pan.value, 'elastic');
    } else {
      const view = viewRef.current;
      applyView({tx: view.tx + dx, ty: view.ty + dy, zoom: view.zoom});
    }
  };

  const followCursor = (before: CursorPoint, after: CursorPoint, canvas: HTMLCanvasElement): void => {
    const vis = visibleRect();
    if (!vis) {
      return;
    }
    const cx = (vis.left + vis.right) / 2;
    const cy = (vis.top + vis.bottom) / 2;
    let panX = 0;
    let panY = 0;
    if (after.x > before.x && after.x > cx && vis.right < canvas.width - 1) {
      panX = (before.x - after.x) * vis.scale;
    } else if (after.x < before.x && after.x < cx && vis.left > 0) {
      panX = (before.x - after.x) * vis.scale;
    }
    if (after.y > before.y && after.y > cy && vis.bottom < canvas.height - 1) {
      panY = (before.y - after.y) * vis.scale;
    } else if (after.y < before.y && after.y < cy && vis.top > 0) {
      panY = (before.y - after.y) * vis.scale;
    }
    if (panX !== 0 || panY !== 0) {
      panBy(panX, panY);
    }
  };

  const visibleRect = (): VisibleRect | null => {
    const canvas = resolveCanvas();
    const host = canvas?.parentElement;
    if (!canvas || !host) {
      return null;
    }
    const box = readHostBox(host);
    const rect = host.getBoundingClientRect();
    const start = canvasPoint(rect.left + box.padLeft, rect.top + box.padTop);
    const end = canvasPoint(rect.left + box.padLeft + box.viewWidth, rect.top + box.padTop + box.viewHeight);
    const content = resolveRdpContentBox(canvas);
    if (!start || !end || !content) {
      return null;
    }
    return {left: start.x, top: start.y, right: end.x, bottom: end.y, scale: content.scale};
  };

  const beginPinch = (points: Iterable<CursorPoint>): void => {
    cancelMotion(false);
    const pair = twoPoints(points);
    const canvas = resolveCanvas();
    const host = canvas?.parentElement;
    if (!pair || !canvas || !host) {
      pinchRef.current = null;
      return;
    }
    const [first, second] = pair;
    const rect = host.getBoundingClientRect();
    const dist = Math.hypot(second.x - first.x, second.y - first.y);
    const view = rawView(viewRef.current);
    pinchCenterRef.current = {x: (first.x + second.x) / 2, y: (first.y + second.y) / 2};
    pinchRef.current = {
      anchorX: ((first.x + second.x) / 2 - rect.left - canvas.offsetLeft - view.tx) / view.zoom,
      anchorY: ((first.y + second.y) / 2 - rect.top - canvas.offsetTop - view.ty) / view.zoom,
      distance: dist,
      zoom: view.zoom,
    };
  };

  const pinchTo = (points: Iterable<CursorPoint>): void => {
    const pinch = pinchRef.current;
    const pair = twoPoints(points);
    const canvas = resolveCanvas();
    const host = canvas?.parentElement;
    if (!pinch || !pair || !canvas || !host || pinch.distance === 0) {
      return;
    }
    const [first, second] = pair;
    const dist = Math.hypot(second.x - first.x, second.y - first.y);
    if (dist === 0) {
      return;
    }
    const rect = host.getBoundingClientRect();
    const midX = (first.x + second.x) / 2 - rect.left;
    const midY = (first.y + second.y) / 2 - rect.top;
    const rawZoom = pinch.zoom * dist / pinch.distance;
    const min = minimumZoom(canvas, readHostBox(host));
    const max = mobileZoomMax();
    const bounded = Math.max(min, Math.min(max, rawZoom));
    const zoom = bounded + rubber(rawZoom - bounded, rawZoom < min ? min * 0.14 : max * 0.1);
    pinchCenterRef.current = {x: (first.x + second.x) / 2, y: (first.y + second.y) / 2};
    // Scale resistance is applied here to keep the content under the fingers;
    // applyView only needs translation resistance for this already-soft scale.
    applyView({tx: midX - canvas.offsetLeft - pinch.anchorX * zoom, ty: midY - canvas.offsetTop - pinch.anchorY * zoom, zoom}, 'elastic');
  };

  const endPinch = (): void => {
    pinchRef.current = null;
  };

  const relayout = (): void => {
    cancelMotion(false);
    const canvas = resolveCanvas();
    const host = canvas?.parentElement;
    const center = centerRef.current;
    if (!canvas || !host || !center) {
      applyView(viewRef.current);
      return;
    }
    const box = readHostBox(host);
    const zoom = Math.max(minimumZoom(canvas, box), Math.min(mobileZoomMax(), viewRef.current.zoom));
    const content = readCanvasBox(canvas);
    applyView({
      tx: box.padLeft + box.viewWidth / 2 - canvas.offsetLeft - (content.insetLeft + center.x * content.width) * zoom,
      ty: box.padTop + box.viewHeight / 2 - canvas.offsetTop - (content.insetTop + center.y * content.height) * zoom,
      zoom,
    });
  };
  const fit = (): void => {
    cancelMotion(false);
    const canvas = resolveCanvas();
    const host = canvas?.parentElement;
    if (!canvas || !host) {
      return;
    }
    const box = readHostBox(host);
    const zoom = minimumZoom(canvas, box);
    const content = readCanvasBox(canvas);
    applyView({
      tx: box.padLeft + (box.viewWidth - content.width * zoom) / 2 - canvas.offsetLeft - content.insetLeft * zoom,
      ty: box.padTop + (box.viewHeight - content.height * zoom) / 2 - canvas.offsetTop - content.insetTop * zoom,
      zoom,
    });
  };
  const actualSize = (): void => {
    cancelMotion(false);
    const canvas = resolveCanvas();
    const host = canvas?.parentElement;
    if (!canvas || !host || !canvas.offsetWidth) {
      return;
    }
    const previous = viewRef.current;
    const zoom = Math.max(1, canvas.width / readCanvasBox(canvas).width / (window.devicePixelRatio || 1));
    const factor = zoom / previous.zoom;
    const box = readHostBox(host);
    const x = box.padLeft + box.viewWidth / 2 - canvas.offsetLeft;
    const y = box.padTop + box.viewHeight / 2 - canvas.offsetTop;
    applyView({tx: x - (x - previous.tx) * factor, ty: y - (y - previous.ty) * factor, zoom});
  };

  const reset = (canvas: HTMLCanvasElement): void => {
    cancelMotion(false);
    pinchRef.current = null;
    centerRef.current = null;
    viewRef.current = identityView();
    canvas.style.transform = '';
    canvas.style.transformOrigin = '';
    publishFocus(null);
  };

  useEffect(() => {
    const preference = window.matchMedia('(prefers-reduced-motion: reduce)');
    const changed = (): void => {
      if (preference.matches) {
        cancelMotion();
      }
    };
    preference.addEventListener('change', changed);
    return () => {
      preference.removeEventListener('change', changed);
      cancelMotion(false);
    };
  }, [canvasRef]);

  return { reset, fit, actualSize, panBy, pinchTo, relayout, endPinch, beginPinch, beginPan, finishGesture, cancelMotion, canvasPoint, visibleRect, followCursor };
}
