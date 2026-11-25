import type {Terminal as XTerm} from '@xterm/xterm';
import {queryXtermScreen} from '@/page/common/util/terminal.ts';

const RECENT_OUTPUT_ROWS = 40;
const DISPLAY_TYPED_MAX_CHARS = 48;
const DEBOUNCE_MIN_MS = 80;
const DEBOUNCE_MAX_MS = 220;
const DEBOUNCE_DEFAULT_MS = 120;
const DEBOUNCE_IKI_WINDOW = 8;
const PANEL_MAX_ROWS = 7;
const PANEL_SKELETON_ROWS = 3;
const PANEL_ROW_HEIGHT = 28;
const PANEL_CHROME_HEIGHT = 66;
const PANEL_BLOCK_MAX_LINES = 8;

export interface SuggestAnchor {
    left: number;
    top: number;
}

function resolveCursorRow(terminal: XTerm): number {
    const buffer = terminal.buffer.active;
    return buffer.baseY + buffer.cursorY;
}

export function readCurrentLine(terminal: XTerm): string {
    const buffer = terminal.buffer.active;
    const row = resolveCursorRow(terminal);
    let start = row;
    while (start > 0 && buffer.getLine(start)?.isWrapped) {
        start--;
    }
    let text = '';
    for (let y = start; y < row; y++) {
        text += buffer.getLine(y)?.translateToString(false) ?? '';
    }
    return text + (buffer.getLine(row)?.translateToString(false, 0, buffer.cursorX) ?? '');
}

export function readRecentOutput(terminal: XTerm): string {
    const buffer = terminal.buffer.active;
    const row = resolveCursorRow(terminal);
    const start = Math.max(0, row - RECENT_OUTPUT_ROWS + 1);
    const end = Math.min(buffer.length - 1, row + RECENT_OUTPUT_ROWS);
    const lines: string[] = [];
    for (let y = start; y <= end; y++) {
        lines.push(buffer.getLine(y)?.translateToString(true) ?? '');
    }
    return lines.join('\n').trimEnd();
}

export function isAltScreen(terminal: XTerm): boolean {
    return terminal.buffer.active.type === 'alternate';
}

export function isMouseCsi(data: string): boolean {
    return data.startsWith('\u001b[<') || data.startsWith('\u001b[M');
}

export function isVimInsert(terminal: XTerm): boolean {
    const buffer = terminal.buffer.active;
    for (let i = 1; i <= 3; i++) {
        const y = buffer.baseY + terminal.rows - i;
        if (y < 0) {
            continue;
        }
        const text = buffer.getLine(y)?.translateToString(true) ?? '';
        if (/--\s*(INSERT|REPLACE|插入|替换)/.test(text)) {
            return true;
        }
    }
    return false;
}

function clipToRowWidth(terminal: XTerm, text: string): string {
    const available = terminal.cols - terminal.buffer.active.cursorX;
    if (available <= 0) {
        return '';
    }
    return text.slice(0, available);
}

export function resolveTypedPreview(currentLine: string): string {
    let cut = -1;
    for (let i = currentLine.length - 1; i >= 0; i--) {
        const ch = currentLine[i];
        if (ch !== '$' && ch !== '#' && ch !== '>' && ch !== '%') {
            continue;
        }
        cut = i + 1;
        if (currentLine[cut] === ' ') {
            cut += 1;
        }
        break;
    }
    const typed = cut < 0 ? currentLine : currentLine.slice(cut);
    if (typed.length <= DISPLAY_TYPED_MAX_CHARS) {
        return typed;
    }
    return `…${typed.slice(typed.length - DISPLAY_TYPED_MAX_CHARS)}`;
}

export function resolveAnchor(terminal: XTerm, host: HTMLElement, panelWidth: number, panelHeight: number): SuggestAnchor | null {
    const screen = queryXtermScreen(host);
    if (screen === null) {
        return null;
    }
    const hostRect = host.getBoundingClientRect();
    const screenRect = screen.getBoundingClientRect();
    if (terminal.cols <= 0 || terminal.rows <= 0) {
        return null;
    }
    const cellWidth = screenRect.width / terminal.cols;
    const cellHeight = screenRect.height / terminal.rows;
    const offsetLeft = screenRect.left - hostRect.left;
    const offsetTop = screenRect.top - hostRect.top;
    const buffer = terminal.buffer.active;

    const rawLeft = offsetLeft + buffer.cursorX * cellWidth;
    const left = Math.max(0, Math.min(rawLeft, hostRect.width - panelWidth));

    const below = offsetTop + (buffer.cursorY + 1) * cellHeight + 4;
    const above = offsetTop + buffer.cursorY * cellHeight - panelHeight - 4;
    const top = below + panelHeight <= hostRect.height || above < 0 ? below : above;
    return {left, top};
}

export interface GhostHandle {
    render(text: string): void;
    clear(): void;
}

export function resolveGhostText(text: string): string {
    let i = 0;
    while (i < text.length && (text[i] === '\n' || text[i] === '\r')) {
        i++;
    }
    const rest = text.slice(i);
    const newline = rest.indexOf('\n');
    if (newline < 0) {
        return rest;
    }
    return rest.slice(0, newline);
}

export function resolveOverlapSuffix(before: string, suggestion: string): string {
    const max = Math.min(before.length, suggestion.length);
    for (let i = max; i > 0; i--) {
        if (suggestion.startsWith(before.slice(before.length - i))) {
            return suggestion.slice(i);
        }
    }
    return suggestion;
}

export function mapSuggestList(currentLine: string, items: string[]): string[] {
    return items.map(item => resolveOverlapSuffix(currentLine, item)).filter(item => item.length > 0);
}

export function resolvePanelHeight(list: string[], loading: boolean): number {
    if (loading && list.length === 0) {
        return PANEL_SKELETON_ROWS * PANEL_ROW_HEIGHT + PANEL_CHROME_HEIGHT;
    }
    const shown = list.slice(0, PANEL_MAX_ROWS);
    if (shown.length === 0) {
        return PANEL_SKELETON_ROWS * PANEL_ROW_HEIGHT + PANEL_CHROME_HEIGHT;
    }
    let body = 0;
    for (const item of shown) {
        const lines = item.split('\n').length;
        body += (lines > PANEL_BLOCK_MAX_LINES ? PANEL_BLOCK_MAX_LINES : lines) * PANEL_ROW_HEIGHT;
    }
    return body + PANEL_CHROME_HEIGHT;
}

export function resolveDebounceMs(intervals: number[]): number {
    const recent = intervals.filter(ms => ms > 0 && ms < 2000).slice(-DEBOUNCE_IKI_WINDOW);
    if (recent.length === 0) {
        return DEBOUNCE_DEFAULT_MS;
    }
    const sorted = [...recent].sort((a, b) => a - b);
    const idle = Math.round(sorted[Math.floor(sorted.length / 2)] * 2);
    return Math.min(DEBOUNCE_MAX_MS, Math.max(DEBOUNCE_MIN_MS, idle));
}

export function resolveAcceptInput(text: string, altScreen: boolean, bracketedPaste: boolean): string | null {
    if (!text.includes('\n')) {
        return text;
    }
    if (altScreen) {
        return text;
    }
    if (bracketedPaste) {
        return `\x1b[200~${text}\x1b[201~`;
    }
    return null;
}

export function createGhost(terminal: XTerm, host: HTMLElement): GhostHandle {
    let node: HTMLDivElement | null = null;

    const clear = (): void => {
        if (node === null) {
            return;
        }
        node.remove();
        node = null;
    };

    const render = (text: string): void => {
        clear();
        const clipped = clipToRowWidth(terminal, resolveGhostText(text));
        if (!clipped) {
            return;
        }
        const screen = queryXtermScreen(host);
        if (screen === null) {
            return;
        }
        const hostRect = host.getBoundingClientRect();
        const screenRect = screen.getBoundingClientRect();
        if (terminal.cols <= 0 || terminal.rows <= 0) {
            return;
        }
        const cellWidth = screenRect.width / terminal.cols;
        const cellHeight = screenRect.height / terminal.rows;
        const el = document.createElement('div');
        el.className = 'tm-suggest-ghost';
        el.textContent = clipped;
        el.style.left = `${screenRect.left - hostRect.left + terminal.buffer.active.cursorX * cellWidth}px`;
        el.style.top = `${screenRect.top - hostRect.top + terminal.buffer.active.cursorY * cellHeight}px`;
        el.style.height = `${cellHeight}px`;
        el.style.lineHeight = `${cellHeight}px`;
        host.appendChild(el);
        node = el;
    };

    return {render, clear};
}
