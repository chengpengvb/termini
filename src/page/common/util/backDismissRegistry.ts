export interface BackDismissEntry {
    id: string;
    dismiss: () => void;
}

export interface TmBackHistoryState {
    tmBack: true;
}

let seq = 0;
let releaseQueued = false;
let syncingHistory = false;
let historyTrapArmed = false;
let rootTrapEnabled = false;
const stack: BackDismissEntry[] = [];
const TM_BACK_STATE: TmBackHistoryState = {tmBack: true};

function isTmBackState(state: unknown): state is TmBackHistoryState {
    return (state as TmBackHistoryState).tmBack;
}

function pushHistoryTrap(): void {
    if (historyTrapArmed || syncingHistory) {
        return;
    }
    historyTrapArmed = true;
    syncingHistory = true;
    history.pushState(TM_BACK_STATE, '');
    syncingHistory = false;
}

function releaseHistoryTrap(): void {
    if (!historyTrapArmed || syncingHistory) {
        return;
    }
    if (!isTmBackState(history.state)) {
        historyTrapArmed = false;
        return;
    }
    syncingHistory = true;
    historyTrapArmed = false;
    history.back();
    window.setTimeout(() => {
        syncingHistory = false;
    }, 0);
}

function syncHistoryTrap(): void {
    if (stack.length > 0 || rootTrapEnabled) {
        pushHistoryTrap();
        return;
    }
    if (releaseQueued) {
        return;
    }
    releaseQueued = true;
    queueMicrotask(() => {
        releaseQueued = false;
        if (stack.length > 0 || rootTrapEnabled) {
            pushHistoryTrap();
            return;
        }
        releaseHistoryTrap();
    });
}

function removeEntry(entry: BackDismissEntry): boolean {
    const index = stack.lastIndexOf(entry);
    if (index < 0) {
        return false;
    }
    stack.splice(index, 1);
    return true;
}

export function pushBackDismiss(dismiss: () => void, id?: string): () => void {
    const entry: BackDismissEntry = {
        dismiss,
        id: id ?? `back-${++seq}`,
    };
    stack.push(entry);
    syncHistoryTrap();
    return () => {
        if (!removeEntry(entry)) {
            return;
        }
        syncHistoryTrap();
    };
}

export function peekBackDismiss(): BackDismissEntry | null {
    if (stack.length === 0) {
        return null;
    }
    return stack[stack.length - 1];
}

export function dismissTopBack(): boolean {
    const top = peekBackDismiss();
    if (!top) {
        return false;
    }
    removeEntry(top);
    top.dismiss();
    syncHistoryTrap();
    return true;
}

export function backDismissDepth(): number {
    return stack.length;
}

export function installBackDismissHistoryTrap(): () => void {
    rootTrapEnabled = true;
    const onPopState = (): void => {
        if (syncingHistory) {
            return;
        }
        historyTrapArmed = false;
        if (stack.length > 0) {
            dismissTopBack();
            return;
        }
        pushHistoryTrap();
    };
    window.addEventListener('popstate', onPopState);
    pushHistoryTrap();
    return () => {
        window.removeEventListener('popstate', onPopState);
        rootTrapEnabled = false;
        releaseHistoryTrap();
    };
}
