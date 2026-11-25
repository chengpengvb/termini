import {createElement} from 'react';
import {createRoot, type Root} from 'react-dom/client';
import {ServicesProvider} from '@/common/context/ServiceContext.tsx';
import type {ChatReference} from '@/common/model/dto/ChatReference.ts';
import {armPointerDrag} from '@/common/components/tab/util/armMouseDrag.ts';
import {ReferenceChip} from '@/page/common/util/reference/ReferenceChip.tsx';

const DRAG_START_DISTANCE_PX = 5;
const GHOST_CURSOR_OFFSET_X = 12;
const GHOST_CURSOR_OFFSET_Y = 12;
const BODY_DRAG_CLASS = 'tm-agent-drag-active';

interface ReferenceDropZone {
    el: HTMLElement;
    onEnter: () => void;
    onLeave: () => void;
    onOver: (clientX: number, clientY: number) => void;
    onDrop: (ref: ChatReference, clientX: number, clientY: number) => void;
}

interface GhostHandle {
    element: HTMLDivElement;
    root: Root;
}

const dropZones: ReferenceDropZone[] = [];
let activeReference: ChatReference | null = null;
let ghost: GhostHandle | null = null;
let hoverZone: ReferenceDropZone | null = null;

function createGhost(ref: ChatReference): GhostHandle {
    const element = document.createElement('div');
    element.className = 'tm-agent-drag-ghost';
    document.body.appendChild(element);
    const root = createRoot(element);
    root.render(createElement(ServicesProvider, null, createElement(ReferenceChip, {kind: ref.kind, id: ref.id, interactive: false, onRemove: null})));
    return {element, root};
}

function moveGhost(clientX: number, clientY: number): void {
    if (!ghost) {
        return;
    }
    ghost.element.style.left = `${clientX + GHOST_CURSOR_OFFSET_X}px`;
    ghost.element.style.top = `${clientY + GHOST_CURSOR_OFFSET_Y}px`;
}

function removeGhost(): void {
    if (!ghost) {
        return;
    }
    const handle = ghost;
    ghost = null;
    handle.root.unmount();
    handle.element.remove();
}

function findDropZone(clientX: number, clientY: number): ReferenceDropZone | null {
    for (const zone of dropZones) {
        const rect = zone.el.getBoundingClientRect();
        if (clientX < rect.left || clientX > rect.right) {
            continue;
        }
        if (clientY < rect.top || clientY > rect.bottom) {
            continue;
        }
        return zone;
    }
    return null;
}

function updateHoverZone(next: ReferenceDropZone | null): void {
    if (hoverZone === next) {
        return;
    }
    if (hoverZone) {
        hoverZone.onLeave();
    }
    hoverZone = next;
    if (hoverZone) {
        hoverZone.onEnter();
    }
}

function resetDragState(): void {
    activeReference = null;
    updateHoverZone(null);
    removeGhost();
    document.body.classList.remove(BODY_DRAG_CLASS);
}

export function armReferenceDrag(ref: ChatReference, pointerId: number, startX: number, startY: number): () => void {
    return armPointerDrag(pointerId, startX, startY, DRAG_START_DISTANCE_PX,
        (x, y) => {
            activeReference = ref;
            ghost = createGhost(ref);
            document.body.classList.add(BODY_DRAG_CLASS);
            moveGhost(x, y);
            updateHoverZone(findDropZone(x, y));
        },
        (x, y) => {
            if (!activeReference) {
                return;
            }
            moveGhost(x, y);
            const zone = findDropZone(x, y);
            updateHoverZone(zone);
            if (zone) {
                zone.onOver(x, y);
            }
        },
        (x, y) => {
            if (!activeReference) {
                return;
            }
            const droppedRef = activeReference;
            const zone = findDropZone(x, y);
            resetDragState();
            if (!zone) {
                return;
            }
            zone.onDrop(droppedRef, x, y);
        },
    );
}

export function registerReferenceDropZone(zone: ReferenceDropZone): () => void {
    dropZones.push(zone);
    return () => {
        const idx = dropZones.indexOf(zone);
        if (idx < 0) {
            return;
        }
        dropZones.splice(idx, 1);
    };
}
