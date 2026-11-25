import {useSyncExternalStore} from 'react';

export interface TaskPhaseSnapshot {
    running: boolean;
    startedAt: number | null;
}

type Listener = () => void;

const listeners = new Set<Listener>();
const snapshots = new Map<string, TaskPhaseSnapshot>();

function emit(): void {
    for (const listener of listeners) {
        listener();
    }
}

export function publishTaskPhase(itemId: string, snapshot: TaskPhaseSnapshot): void {
    const current = snapshots.get(itemId);
    if (current && current.running === snapshot.running && current.startedAt === snapshot.startedAt) {
        return;
    }
    snapshots.set(itemId, snapshot);
    emit();
}

export function dropTaskPhase(itemId: string): void {
    if (snapshots.delete(itemId)) {
        emit();
    }
}

function subscribe(listener: Listener): () => void {
    listeners.add(listener);
    return () => {
        listeners.delete(listener);
    };
}

export interface TaskGroupState {
    running: boolean;
    startedAt: number | null;
}

export function useTaskGroupState(itemIds: string[]): TaskGroupState {
    const key = itemIds.join('|');
    return useSyncExternalStore(
        subscribe,
        () => computeGroupState(itemIds, key),
        () => computeGroupState(itemIds, key),
    );
}

const cache = new Map<string, {signature: string; state: TaskGroupState}>();

function computeGroupState(itemIds: string[], key: string): TaskGroupState {
    let running = false;
    let startedAt: number | null = null;
    const parts: string[] = [];
    for (const id of itemIds) {
        const snapshot = snapshots.get(id);
        if (!snapshot) {
            parts.push('-');
            continue;
        }
        parts.push(`${snapshot.running ? 1 : 0}:${snapshot.startedAt ?? ''}`);
        running = running || snapshot.running;
        if (snapshot.startedAt !== null && (startedAt === null || snapshot.startedAt < startedAt)) {
            startedAt = snapshot.startedAt;
        }
    }
    const signature = parts.join(',');
    const cached = cache.get(key);
    if (cached && cached.signature === signature) {
        return cached.state;
    }
    const state = {running, startedAt};
    cache.set(key, {signature, state});
    return state;
}
