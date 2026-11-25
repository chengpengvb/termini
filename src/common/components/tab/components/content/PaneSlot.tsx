import type {ReactNode} from 'react';
import {useLayoutEffect, useRef} from 'react';

interface PaneSlotProps {
    host: HTMLDivElement;
    paneId: string;
}

const scrollMemoryByHost = new WeakMap<HTMLDivElement, Map<HTMLElement, [number, number]>>();

function recordScroll(host: HTMLDivElement, event: Event): void {
    const el = event.target;
    if (!(el instanceof HTMLElement)) {
        return;
    }
    let memory = scrollMemoryByHost.get(host);
    if (!memory) {
        memory = new Map();
        scrollMemoryByHost.set(host, memory);
    }
    memory.set(el, [el.scrollTop, el.scrollLeft]);
}

function restoreScrolls(host: HTMLDivElement): void {
    const memory = scrollMemoryByHost.get(host);
    if (!memory) {
        return;
    }
    for (const [el, [top, left]] of memory) {
        if (!host.contains(el)) {
            memory.delete(el);
            continue;
        }
        el.scrollTop = top;
        el.scrollLeft = left;
    }
}

export function PaneSlot(props: PaneSlotProps): ReactNode {
    const {host} = props;
    const slotRef = useRef<HTMLDivElement | null>(null);

    useLayoutEffect(() => {
        const slot = slotRef.current;
        if (!slot) {
            return;
        }
        const onScroll = (event: Event): void => recordScroll(host, event);
        slot.appendChild(host);
        restoreScrolls(host);
        host.addEventListener('scroll', onScroll, true);
        return () => {
            host.removeEventListener('scroll', onScroll, true);
            if (host.parentNode === slot) {
                slot.removeChild(host);
            }
        };
    }, [host]);

    return <div ref={slotRef} style={{ width: '100%', height: '100%' }} />;
}
