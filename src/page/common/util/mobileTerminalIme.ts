import type {Terminal} from '@xterm/xterm';

const GUARD = '\u2060'.repeat(128);

export interface MobileTerminalImeHandle {
    dispose(): void;
    externalInput(): void;
}

function runIme(work: () => void): void {
    try {
        work();
    } catch (error) {
        console.warn(error);
    }
}

export function attachMobileTerminalIme(terminal: Terminal): MobileTerminalImeHandle {
    const area = terminal.textarea;
    const host = area?.parentElement;
    if (!area || !host) {
        throw new Error('Terminal IME requires an opened terminal');
    }
    let text = '';
    let softKey = '';
    let snapshot = '';
    let composing = false;
    let finishing = false;
    let physical = false;
    let disposed = false;
    let emitting = false;
    let soft229 = false;
    let inserted = false;
    let lastInput: string | null = null;
    let compositionEcho: string | null = null;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const segmenter = new Intl.Segmenter(undefined, {granularity: 'grapheme'});
    const clean = (value: string): string => value.replace(/^\u2060+/, '');
    const count = (value: string): number => [...segmenter.segment(value)].length;
    const emit = (value: string): void => {
        if (!value) {
            return;
        }
        emitting = true;
        try {
            terminal.input(value, true);
        } catch (error) {
            console.warn(error);
        } finally {
            emitting = false;
        }
    };
    const refill = (): void => {
        if (disposed || composing || finishing || physical || area.getAttribute('inputmode') === 'none') {
            return;
        }
        text = text.slice(-256);
        area.value = GUARD + text;
        area.setSelectionRange(area.value.length, area.value.length);
    };
    const idle = (): void => {
        clearTimeout(timer);
        timer = setTimeout(refill, 350);
    };
    const stop = (event: Event): void => event.stopImmediatePropagation();
    const reconcile = (next: string): void => {
        let prefix = 0;
        const oldPoints = [...text];
        const nextPoints = [...next];
        while (prefix < oldPoints.length && prefix < nextPoints.length && oldPoints[prefix] === nextPoints[prefix]) {
            prefix++;
        }
        emit('\x7f'.repeat(count(oldPoints.slice(prefix).join(''))) + nextPoints.slice(prefix).join(''));
        text = next;
    };
    const start = (event: Event): void => {
        if (event.target !== area) {
            return;
        }
        clearTimeout(timer);
        if (!finishing) {
            area.value = '';
            text = '';
        }
        composing = true;
        finishing = false;
        soft229 = false;
        inserted = true;
        compositionEcho = null;
    };
    const end = (event: Event): void => {
        if (event.target !== area) {
            return;
        }
        composing = false;
        finishing = true;
        queueMicrotask(() => {
            if (disposed) {
                return;
            }
            clearTimeout(timer);
            timer = setTimeout(() => {
                finishing = false;
                refill();
            }, 0);
        });
    };
    const keydown = (event: KeyboardEvent): void => {
        if (event.target !== area || composing || event.isComposing || finishing) {
            return;
        }
        soft229 = event.keyCode === 229;
        softKey = event.key;
        inserted = lastInput === event.key;
        lastInput = null;
        compositionEcho = null;
        physical = area.getAttribute('inputmode') === 'none' && !soft229;
        if (soft229 || (!physical && event.key === 'Backspace')) {
            stop(event);
        } else if (!physical && event.key.length === 1 && !event.ctrlKey && !event.altKey && !event.metaKey) {
            stop(event);
        } else {
            text = '';
            area.value = '';
            idle();
        }
    };
    const keypress = (event: KeyboardEvent): void => {
        if (event.target === area && !composing && !finishing && !physical && event.key.length === 1 && !event.ctrlKey && !event.altKey && !event.metaKey) {
            stop(event);
        }
    };
    const keyup = (event: KeyboardEvent): void => {
        if (event.target !== area || composing || event.isComposing || finishing) {
            return;
        }
        if (soft229 && event.key === softKey && !inserted && event.key.length === 1) {
            emit(event.key);
            text += event.key;
            refill();
        }
        if (soft229 && event.key !== softKey) {
            return;
        }
        if (soft229) {
            stop(event);
        }
        physical = false;
        soft229 = false;
        lastInput = null;
        idle();
    };
    const before = (event: InputEvent): void => {
        if (event.target !== area || composing || finishing || event.isComposing) {
            return;
        }
        snapshot = clean(area.value);
        if (physical) {
            return;
        }
        if (['deleteContentBackward', 'deleteWordBackward', 'deleteContentForward'].includes(event.inputType)) {
            stop(event);
            if (!event.cancelable) {
                return;
            }
            event.preventDefault();
            const guard = area.value.length - snapshot.length;
            const from = Math.max(0, area.selectionStart - guard);
            const to = Math.max(0, area.selectionEnd - guard);
            const forward = event.inputType === 'deleteContentForward';
            let deleted = snapshot.slice(from, to);
            if (!deleted) {
                if (event.inputType === 'deleteWordBackward') {
                    deleted = snapshot.slice(0, from).match(/\S+\s*$/u)?.[0] ?? '';
                } else {
                    const segments = [...segmenter.segment(forward ? snapshot.slice(to) : snapshot.slice(0, from))];
                    deleted = segments[forward ? 0 : segments.length - 1]?.segment ?? '';
                }
            }
            emit((forward ? '\x1b[3~' : '\x7f').repeat(Math.max(1, count(deleted))));
            const begin = to > from || forward ? from : Math.max(0, from - deleted.length);
            const finish = to > from || !forward ? to : to + deleted.length;
            area.setRangeText('', guard + begin, guard + finish, 'end');
            text = clean(area.value);
            idle();
        }
    };
    const input = (event: InputEvent): void => {
        if (event.target !== area || composing || event.isComposing) {
            return;
        }
        if (finishing) {
            stop(event);
            return;
        }
        if (physical) {
            return;
        }
        stop(event);
        const next = clean(area.value);
        if (compositionEcho && event.data === compositionEcho && (next === text || next === text + compositionEcho)) {
            compositionEcho = null;
            refill();
            return;
        }
        if (event.inputType.startsWith('delete')) {
            emit((event.inputType === 'deleteContentForward' ? '\x1b[3~' : '\x7f').repeat(Math.max(1, count(snapshot) - count(next))));
            text = next;
        } else {
            if (next === text && event.data && event.inputType === 'insertText') {
                emit(event.data);
                text += event.data;
                refill();
            } else {
                reconcile(next);
            }
            lastInput = event.data;
            inserted = true;
        }
        idle();
    };
    const data = terminal.onData(value => {
        if (composing || finishing) {
            text += value;
            compositionEcho = value;
        }
    });
    const onStart = (event: Event): void => runIme(() => start(event));
    const onEnd = (event: Event): void => runIme(() => end(event));
    const onKeyDown = (event: Event): void => runIme(() => keydown(event as KeyboardEvent));
    const onKeyPress = (event: Event): void => runIme(() => keypress(event as KeyboardEvent));
    const onKeyUp = (event: Event): void => runIme(() => keyup(event as KeyboardEvent));
    const onBefore = (event: Event): void => runIme(() => before(event as InputEvent));
    const onInput = (event: Event): void => runIme(() => input(event as InputEvent));
    const onFocus = (): void => runIme(refill);
    host.addEventListener('compositionstart', onStart, true);
    host.addEventListener('compositionend', onEnd, true);
    host.addEventListener('keydown', onKeyDown, true);
    host.addEventListener('keypress', onKeyPress, true);
    host.addEventListener('keyup', onKeyUp, true);
    host.addEventListener('beforeinput', onBefore, true);
    host.addEventListener('input', onInput, true);
    area.addEventListener('focus', onFocus);
    refill();
    return {
        externalInput: () => {
            runIme(() => {
                if (emitting || composing || finishing || disposed) {
                    return;
                }
                text = '';
                snapshot = '';
                lastInput = null;
                compositionEcho = null;
                area.value = '';
                refill();
            });
        },
        dispose: () => {
            disposed = true;
            clearTimeout(timer);
            data.dispose();
            host.removeEventListener('compositionstart', onStart, true);
            host.removeEventListener('compositionend', onEnd, true);
            host.removeEventListener('keydown', onKeyDown, true);
            host.removeEventListener('keypress', onKeyPress, true);
            host.removeEventListener('keyup', onKeyUp, true);
            host.removeEventListener('beforeinput', onBefore, true);
            host.removeEventListener('input', onInput, true);
            area.removeEventListener('focus', onFocus);
        },
    };
}
