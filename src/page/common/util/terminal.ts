import {WebglAddon} from '@xterm/addon-webgl';
import {translate} from '@/common/i18n/i18n.ts';
import {isEmpty} from '@/common/util/StringUtil.ts';
import {WindowSize} from '@/common/model/dto/WindowSize.ts';
import type {ITheme, ITerminalOptions, Terminal} from '@xterm/xterm';
import {PANE_DRAG_MODE} from "@/common/components/tab/model/PaneDrag.ts";
import {PaneConfig, TabConfig, TabManagerService} from "@/common/components/tab";
import type {TerminalScreen} from '@/page/common/dto/terminal/TerminalHandle.ts';
import {terminalTouchViewport} from '@/page/common/util/terminalTouchViewport.ts';
import {TERMINAL_CURSOR_STYLE, TERMINAL_HANDLE_FACE, TERMINAL_POINTER, TERMINAL_POINTER_MODE, TERMINAL_SELECT_HANDLE} from '@/common/const/Const.ts';

export function resolveTerminalTheme() {
    const css = getComputedStyle(document.documentElement);
    const terminalBg = css.getPropertyValue('--tm-terminal-bg').trim();
    const terminalFg = css.getPropertyValue('--tm-terminal-fg').trim();
    const activeBg = css.getPropertyValue('--tm-terminal-search-active-bg').trim();
    const scrollbarThumbBg = css.getPropertyValue('--tm-bg-selected').trim();
    const scrollbarThumbHoverBg = css.getPropertyValue('--tm-bg-hover').trim();
    return {
        background: terminalBg,
        foreground: terminalFg,
        selectionBackground: activeBg,
        selectionForeground: terminalFg,
        selectionInactiveBackground: activeBg,
        scrollbarSliderBackground: scrollbarThumbBg,
        scrollbarSliderHoverBackground: scrollbarThumbHoverBg,
        scrollbarSliderActiveBackground: scrollbarThumbHoverBg,
    };
}

export function resolveTerminalFontFamily(): string {
    const css = getComputedStyle(document.documentElement);
    const fontMono = css.getPropertyValue('--tm-font-mono').trim();
    if (!fontMono) {
        return 'Menlo, Consolas, monospace';
    }
    return fontMono;
}

export function resolveRootFontSizePx(): number {
    const size = Number.parseFloat(getComputedStyle(document.documentElement).fontSize);
    if (!Number.isFinite(size) || size <= 0) {
        return 16;
    }
    return size;
}

export function remToPx(rem: number): number {
    return rem * resolveRootFontSizePx();
}

export function normalizeTerminalFontSizeRem(value: number): number {
    if (!Number.isFinite(value) || value <= 0) {
        return DEFAULT_TERMINAL_FONT_SIZE;
    }
    if (value > 3) {
        return Math.round((value / 16) * 1000) / 1000;
    }
    return Math.round(value * 1000) / 1000;
}

export function resolveTerminalFontSize(): number {
    const prop = getComputedStyle(document.documentElement).getPropertyValue('--tm-terminal-font-size').trim();
    const size = Number.parseFloat(prop);
    if (!Number.isFinite(size) || size <= 0) {
        return remToPx(DEFAULT_TERMINAL_FONT_SIZE);
    }
    if (prop.endsWith('rem')) {
        return remToPx(size);
    }
    return size;
}

export function normalizeTerminalCursorStyle(value: string | null | undefined): string {
    if (value === TERMINAL_CURSOR_STYLE.BAR) {
        return TERMINAL_CURSOR_STYLE.BAR;
    }
    return TERMINAL_CURSOR_STYLE.BLOCK;
}

export function applyTerminalCursorStyle(terminal: Terminal, cursorStyle: string | null | undefined): void {
    const style = normalizeTerminalCursorStyle(cursorStyle);
    const isBar = style === TERMINAL_CURSOR_STYLE.BAR;
    terminal.options.cursorBlink = true;
    terminal.options.cursorWidth = isBar ? 2 : 1;
    terminal.options.cursorStyle = isBar ? 'bar' : 'block';
}

export function resolveTerminalOptions(setting?: TerminalSetting, mobile = false): Partial<ITerminalOptions> {
    const fontSize = setting
        ? remToPx(normalizeTerminalFontSizeRem(setting.fontSize))
        : resolveTerminalFontSize();
    const fontFamily = setting
        ? resolveTerminalFontStack(setting.fontFamily)
        : resolveTerminalFontFamily();
    const cursorStyle = setting
        ? normalizeTerminalCursorStyle(setting.cursorStyle)
        : DEFAULT_TERMINAL_CURSOR_STYLE;
    const isBar = cursorStyle === TERMINAL_CURSOR_STYLE.BAR;
    return {
        fontSize,
        fontFamily,
        lineHeight: 1,
        letterSpacing: 0,
        cursorBlink: true,
        macOptionIsMeta: mobile,
        cursorWidth: isBar ? 2 : 1,
        cursorStyle: isBar ? 'bar' : 'block',
    };
}

export interface TerminalFontOption {
    value: string;
    label: string;
    stack: string;
}

export const SYSTEM_DEFAULT_FONT_ID = 'system-default';

export const TERMINAL_FONT_OPTIONS: TerminalFontOption[] = [
    {value: SYSTEM_DEFAULT_FONT_ID, label: 'system-default', stack: ''},
    {value: 'jetbrains-mono', label: 'JetBrains Mono', stack: "'JetBrains Mono', monospace"},
    {value: 'source-code-pro', label: 'Source Code Pro', stack: "'Source Code Pro', monospace"},
    {value: 'fira-code', label: 'Fira Code', stack: "'Fira Code', monospace"},
    {value: 'cascadia-code', label: 'Cascadia Code', stack: "'Cascadia Code', monospace"},
    {value: 'ibm-plex-mono', label: 'IBM Plex Mono', stack: "'IBM Plex Mono', monospace"},
    {value: 'space-mono', label: 'Space Mono', stack: "'Space Mono', monospace"},
    {value: 'roboto-mono', label: 'Roboto Mono', stack: "'Roboto Mono', monospace"},
    {value: 'ubuntu-mono', label: 'Ubuntu Mono', stack: "'Ubuntu Mono', monospace"},
];

export const DEFAULT_TERMINAL_FONT_SIZE = 0.875;
export const TERMINAL_FONT_SIZE_MIN_REM = 0.5625;
export const TERMINAL_FONT_SIZE_MAX_REM = 1.75;
export const TERMINAL_FONT_SIZE_STEP_REM = 0.0625;
export const DEFAULT_TERMINAL_FONT_ID = SYSTEM_DEFAULT_FONT_ID;

export function resolveTerminalFontStack(fontId: string): string {
    if (fontId === SYSTEM_DEFAULT_FONT_ID) {
        return resolveTerminalFontFamily();
    }
    const found = TERMINAL_FONT_OPTIONS.find(item => item.value === fontId);
    return found ? found.stack : resolveTerminalFontFamily();
}

export function resolveTerminalFontOptions(): TerminalFontOption[] {
    return TERMINAL_FONT_OPTIONS.map(item => {
        if (item.value === SYSTEM_DEFAULT_FONT_ID) {
            return {...item, label: translate('protocol.terminal.systemDefault')};
        }
        return item;
    });
}

export interface TerminalThemeOption {
    id: string;
    name: string;
    subtitleKey: string;
    bg: string;
    lines: string[];
}

export const TERMINAL_THEME_OPTIONS: TerminalThemeOption[] = [
    {id: 'default', name: 'Default', subtitleKey: 'default', bg: '#23272e', lines: ['#7bd0f9', '#e06c75', '#ffd666']},
    {id: 'graphite', name: 'Graphite', subtitleKey: 'graphite', bg: '#1c1c1e', lines: ['#aeaeb2', '#8e8e93', '#f2f2f7']},
    {id: 'midnight', name: 'Midnight', subtitleKey: 'dark', bg: '#12141a', lines: ['#8b93a0', '#6b7280', '#e8eaed']},
    {id: 'zinc', name: 'Zinc', subtitleKey: 'dark', bg: '#18181b', lines: ['#a1a1aa', '#71717a', '#fafafa']},
    {id: 'nightfall', name: 'Nightfall', subtitleKey: 'dark', bg: '#14161a', lines: ['#2ee6a6', '#4c9aff', '#f2c94c']},
    {id: 'daybreak', name: 'Daybreak', subtitleKey: 'light', bg: '#f4f5f7', lines: ['#8b93a0', '#5b6270', '#2a2f36']},
    {id: 'seasalt', name: 'Seasalt', subtitleKey: 'light', bg: '#f2f2f7', lines: ['#8e8e93', '#636366', '#1c1c1e']},
    {id: 'kanagawa-wave', name: 'Kanagawa Wave', subtitleKey: 'dark', bg: '#1f1f28', lines: ['#dca561', '#7fb4ca', '#98bb6c']},
    {id: 'kanagawa-dragon', name: 'Kanagawa Dragon', subtitleKey: 'dark', bg: '#181616', lines: ['#c4746e', '#8a9a7b', '#c4b28a']},
    {id: 'kanagawa-lotus', name: 'Kanagawa Lotus', subtitleKey: 'light', bg: '#f2ecbc', lines: ['#c84053', '#4d699b', '#6f894e']},
    {id: 'hacker-blue', name: 'Hacker Blue', subtitleKey: 'dark', bg: '#05070d', lines: ['#2f7dff', '#57a6ff', '#9fd0ff']},
    {id: 'hacker-green', name: 'Hacker Green', subtitleKey: 'dark', bg: '#050805', lines: ['#17d117', '#4bff4b', '#9dff9d']},
    {id: 'hacker-red', name: 'Hacker Red', subtitleKey: 'dark', bg: '#0d0505', lines: ['#ff2b2b', '#ff6b6b', '#ffb0b0']},
    {id: 'cb-dark', name: 'Clear Dark', subtitleKey: 'highContrastDark', bg: '#1a1b1e', lines: ['#d55e00', '#009e73', '#56b4e9']},
    {id: 'cb-light', name: 'Clear Light', subtitleKey: 'highContrastLight', bg: '#f5f3ef', lines: ['#d55e00', '#007a5e', '#0072b2']},
];

export function resolveTerminalThemeSubtitle(theme: TerminalThemeOption): string {
    return translate(`protocol.terminal.themeSubtitle.${theme.subtitleKey}`);
}

export const DEFAULT_TERMINAL_THEME_ID = 'default';
export const DEFAULT_TERMINAL_CURSOR_STYLE = TERMINAL_CURSOR_STYLE.BLOCK;

export const TERMINAL_CURSOR_OPTIONS = [
    {value: TERMINAL_CURSOR_STYLE.BLOCK, label: 'block'},
    {value: TERMINAL_CURSOR_STYLE.BAR, label: 'bar'},
];

export function resolveTerminalCursorOptions(): { value: string; label: string }[] {
    return TERMINAL_CURSOR_OPTIONS.map(item => {
        if (item.value === TERMINAL_CURSOR_STYLE.BLOCK) {
            return {...item, label: translate('protocol.terminal.cursorBlock')};
        }
        return {...item, label: translate('protocol.terminal.cursorBar')};
    });
}

export interface TerminalSetting {
    themeId: string;
    fontSize: number;
    fontFamily: string;
    cursorStyle: string;
}

export function resolveTerminalSettingKey(deviceId: string, terminalKind: string): string {
    return `${deviceId}:${terminalKind}`;
}

export const TERMINAL_TAB_LOST_CLASS = 'tm-protocol-tab-lost';

export function markPaneTitleLost(tabManagerService: TabManagerService, paneId: string, lost: boolean): void {
    const pane = tabManagerService.getPane(paneId);
    if (!pane) {
        return;
    }
    tabManagerService.updatePaneBar(paneId, {...pane.tabBar, titleClassName: lost ? TERMINAL_TAB_LOST_CLASS : undefined});
}

const TERMINAL_THEME_VARS = [
    'background', 'foreground', 'cursor', 'cursorAccent', 'selectionBackground', 'selectionForeground', 'selectionInactiveBackground',
    'black', 'red', 'green', 'yellow', 'blue', 'magenta', 'cyan', 'white',
    'brightBlack', 'brightRed', 'brightGreen', 'brightYellow', 'brightBlue', 'brightMagenta', 'brightCyan', 'brightWhite',
] as const;

const TERMINAL_THEME_VAR_NAMES: Record<typeof TERMINAL_THEME_VARS[number], string> = {
    background: '--tm-term-bg',
    foreground: '--tm-term-fg',
    cursor: '--tm-term-cursor',
    cursorAccent: '--tm-term-cursor-accent',
    selectionBackground: '--tm-term-selection-bg',
    selectionForeground: '--tm-term-fg',
    selectionInactiveBackground: '--tm-term-selection-bg',
    black: '--tm-term-black',
    red: '--tm-term-red',
    green: '--tm-term-green',
    yellow: '--tm-term-yellow',
    blue: '--tm-term-blue',
    magenta: '--tm-term-magenta',
    cyan: '--tm-term-cyan',
    white: '--tm-term-white',
    brightBlack: '--tm-term-bright-black',
    brightRed: '--tm-term-bright-red',
    brightGreen: '--tm-term-bright-green',
    brightYellow: '--tm-term-bright-yellow',
    brightBlue: '--tm-term-bright-blue',
    brightMagenta: '--tm-term-bright-magenta',
    brightCyan: '--tm-term-bright-cyan',
    brightWhite: '--tm-term-bright-white',
};

export function resolveTerminalThemeColors(themeId: string, host: HTMLElement): ITheme {
    host.setAttribute('data-terminal-theme', themeId);
    const css = getComputedStyle(host);
    const theme: Partial<ITheme> = {};
    for (const key of TERMINAL_THEME_VARS) {
        const value = css.getPropertyValue(TERMINAL_THEME_VAR_NAMES[key]).trim();
        if (value) {
            (theme as Record<string, string>)[key] = value;
        }
    }
    return theme;
}

export function readHardKeyboardConnected(): boolean {
    return getComputedStyle(document.documentElement).getPropertyValue('--tm-hard-keyboard').trim() === '1';
}

export function prepareMobileIme(terminal: Terminal, hardKeyboard: boolean): void {
    const textarea = terminal.textarea;
    if (!textarea) {
        console.warn('xterm textarea missing');
        return;
    }
    textarea.classList.add('tm-terminal-ime');
    textarea.setAttribute('data-tm-ime', 'true');
    textarea.setAttribute('inputmode', hardKeyboard ? 'none' : 'text');
    textarea.setAttribute('enterkeyhint', 'enter');
    textarea.setAttribute('autocorrect', 'off');
    textarea.setAttribute('autocapitalize', 'off');
    textarea.spellcheck = false;
}

export function requestSoftInput(show: boolean): void {
    const native = window.TerminiNative;
    if (!native) {
        return;
    }
    if (show) {
        native.showSoftInput();
        return;
    }
    native.hideSoftInput();
}

export function stampXtermScreen(host: HTMLElement): void {
    const textarea = host.querySelector('textarea');
    const helpers = textarea === null ? null : textarea.parentElement;
    const screen = helpers === null ? null : helpers.parentElement;
    if (!(screen instanceof HTMLElement)) {
        console.warn('xterm screen missing');
        return;
    }
    screen.setAttribute('data-tm-xterm-screen', 'true');
}

export function queryXtermScreen(host: ParentNode): HTMLElement | null {
    const screen = host.querySelector('[data-tm-xterm-screen="true"]');
    if (!(screen instanceof HTMLElement)) {
        return null;
    }
    return screen;
}

export type TerminalCell = { col: number; row: number };
export type TerminalMenuPoint = { x: number; y: number };

export function readTerminalCellMetrics(host: HTMLElement, terminal: Terminal) {
    const screen = queryXtermScreen(host);
    const box = host.parentElement;
    const row = screen instanceof HTMLElement ? screen.querySelector('.xterm-rows > div') : null;
    if (!(screen instanceof HTMLElement) || !(box instanceof HTMLElement)) {
        console.warn('xterm screen missing');
        return null;
    }
    const screenRect = screen.getBoundingClientRect();
    const rowRect = row instanceof HTMLElement ? row.getBoundingClientRect() : screenRect;
    const boxRect = box.getBoundingClientRect();
    const cellW = rowRect.width / terminal.cols;
    const cellH = row instanceof HTMLElement ? rowRect.height : screenRect.height / terminal.rows;
    if (cellW <= 0 || cellH <= 0) {
        console.warn('xterm cell size invalid');
        return null;
    }
    return {
        screenRect,
        boxRect,
        cellW,
        cellH,
        originLeft: rowRect.left - boxRect.left,
        originTop: rowRect.top - boxRect.top,
        viewportY: terminal.buffer.active.viewportY,
        rowLeft: rowRect.left,
        rowTop: rowRect.top,
    };
}

export function resolveTerminalCell(host: HTMLElement, terminal: Terminal, clientX: number, clientY: number): TerminalCell | null {
    const layout = readTerminalCellMetrics(host, terminal);
    if (!layout) {
        return null;
    }
    const col = Math.min(terminal.cols - 1, Math.max(0, Math.floor((clientX - layout.rowLeft) / layout.cellW)));
    const rowInView = Math.min(terminal.rows - 1, Math.max(0, Math.floor((clientY - layout.rowTop) / layout.cellH)));
    return {col, row: layout.viewportY + rowInView};
}

function readTerminalScreenLayout(host: HTMLElement, terminal: Terminal) {
    return readTerminalCellMetrics(host, terminal);
}

function applyTerminalSelect(terminal: Terminal, from: TerminalCell, to: TerminalCell): void {
    const cols = terminal.cols;
    const start = from.row * cols + from.col;
    const end = to.row * cols + to.col;
    const lo = Math.min(start, end);
    const hi = Math.max(start, end);
    terminal.select(lo % cols, Math.floor(lo / cols), hi - lo + 1);
}

function isTerminalLineContent(terminal: Terminal, row: number): boolean {
    const line = terminal.buffer.active.getLine(row);
    if (!line) {
        return false;
    }
    return line.translateToString(true).length > 0;
}

export function readTerminalContentBounds(terminal: Terminal): { first: number; last: number } | null {
    const len = terminal.buffer.active.length;
    let first = -1;
    let last = -1;
    for (let y = 0; y < len; y++) {
        if (isTerminalLineContent(terminal, y)) {
            if (first < 0) {
                first = y;
            }
            last = y;
        }
    }
    if (first < 0) {
        console.warn('terminal content missing');
        return null;
    }
    return {first, last};
}

export function clampTerminalCellToContent(terminal: Terminal, cell: TerminalCell): TerminalCell {
    const bounds = readTerminalContentBounds(terminal);
    if (!bounds) {
        return cell;
    }
    return {
        col: cell.col,
        row: Math.min(bounds.last, Math.max(bounds.first, cell.row)),
    };
}

export function hasTerminalTextAtCell(terminal: Terminal, cell: TerminalCell): boolean {
    const line = terminal.buffer.active.getLine(cell.row);
    let col = cell.col;
    let hit = line?.getCell(col);
    while (hit?.getWidth() === 0 && col > 0) {
        hit = line?.getCell(--col);
    }
    return !!hit && !hit.isInvisible() && hit.getChars().trim().length > 0;
}

export function expandTerminalSelectToWord(terminal: Terminal, cell: TerminalCell): void {
    const line = terminal.buffer.active.getLine(cell.row);
    if (!line) {
        console.warn('terminal line missing');
        return;
    }
    const hit = line.getCell(cell.col);
    if (!hit) {
        console.warn('terminal cell missing');
        return;
    }
    if (hit.getWidth() !== 0 && (hit.getChars() === ' ' || hit.getChars() === '')) {
        applyTerminalSelect(terminal, cell, cell);
        return;
    }
    let text = '';
    const cols = terminal.cols;
    const colAt: number[] = [];
    const indexAt: number[] = [];
    for (let x = 0; x < cols; x++) {
        const item = line.getCell(x);
        if (!item) {
            console.warn('terminal cell missing');
            return;
        }
        if (item.getWidth() === 0) {
            indexAt.push(text.length - 1);
            continue;
        }
        const ch = item.getChars();
        indexAt.push(text.length);
        if (ch.length === 0) {
            continue;
        }
        text += ch;
        for (let i = 0; i < ch.length; i++) {
            colAt.push(x);
        }
    }
    let to = cell.col;
    let from = cell.col;
    const index = indexAt[cell.col];
    const segmenter = new Intl.Segmenter(undefined, {granularity: 'word'});
    for (const part of segmenter.segment(text)) {
        const start = part.index;
        const end = start + part.segment.length;
        if (index >= start && index < end) {
            from = colAt[start];
            to = colAt[end - 1];
            break;
        }
    }
    applyTerminalSelect(terminal, {col: from, row: cell.row}, {col: to, row: cell.row});
}

export function readTerminalSelectionRange(terminal: Terminal): { start: TerminalCell; end: TerminalCell } | null {
    const pos = terminal.getSelectionPosition();
    if (!pos) {
        return null;
    }
    return {
        start: {col: pos.start.x, row: pos.start.y},
        end: {col: pos.end.x, row: pos.end.y},
    };
}

function exclusiveToInclusive(end: TerminalCell, cols: number): TerminalCell {
    if (end.col > 0) {
        return {col: end.col - 1, row: end.row};
    }
    if (end.row === 0) {
        return {col: 0, row: 0};
    }
    return {col: cols - 1, row: end.row - 1};
}

export function readTerminalSelectionAnchor(terminal: Terminal, which: number): TerminalCell | null {
    const range = readTerminalSelectionRange(terminal);
    if (!range) {
        console.warn('terminal selection missing');
        return null;
    }
    if (which === TERMINAL_SELECT_HANDLE.START) {
        return exclusiveToInclusive(range.end, terminal.cols);
    }
    if (which === TERMINAL_SELECT_HANDLE.END) {
        return range.start;
    }
    console.warn('select handle unknown');
    return null;
}

export function readTerminalSelectionEndpoint(terminal: Terminal, which: number): TerminalCell | null {
    const range = readTerminalSelectionRange(terminal);
    if (!range) {
        return null;
    }
    return which === TERMINAL_SELECT_HANDLE.START ? range.start : exclusiveToInclusive(range.end, terminal.cols);
}

export function setTerminalSelectionEndpoint(terminal: Terminal, anchor: TerminalCell, cell: TerminalCell): void {
    applyTerminalSelect(terminal, clampTerminalCellToContent(terminal, cell), anchor);
}

export function scrollTerminalSelectAtEdge(host: HTMLElement, terminal: Terminal, clientY: number): void {
    const layout = readTerminalScreenLayout(host, terminal);
    const bounds = readTerminalContentBounds(terminal);
    if (!layout || !bounds) {
        return;
    }
    const viewportY = terminal.buffer.active.viewportY;
    const edge = layout.cellH;
    if (clientY < layout.screenRect.top + edge && viewportY > bounds.first) {
        terminal.scrollLines(-1);
        return;
    }
    if (clientY > layout.screenRect.bottom - edge && viewportY + terminal.rows - 1 < bounds.last) {
        terminal.scrollLines(1);
    }
}

export function resolveTerminalHandlePlace(
    host: HTMLElement,
    terminal: Terminal,
    cell: TerminalCell,
    face: number,
): { left: number; top: number; visible: boolean; face: number; above?: boolean } {
    const layout = readTerminalScreenLayout(host, terminal);
    if (!layout) {
        return {left: 0, top: 0, visible: false, face};
    }
    const handle = 44;
    const rowInView = cell.row - layout.viewportY;
    const left = Math.max(layout.screenRect.left - layout.boxRect.left + 11,
        Math.min(layout.originLeft + cell.col * layout.cellW, layout.screenRect.right - layout.boxRect.left - 11));
    const top = layout.originTop + (rowInView + 1) * layout.cellH;
    const viewportBottom = (window.visualViewport?.offsetTop ?? 0) + (window.visualViewport?.height ?? window.innerHeight);
    const above = top + handle > Math.min(layout.screenRect.bottom, viewportBottom) - layout.boxRect.top;
    let next = face;
    if (left - handle < 0) {
        next = TERMINAL_HANDLE_FACE.RIGHT;
    } else if (left + handle > layout.boxRect.width) {
        next = TERMINAL_HANDLE_FACE.LEFT;
    }
    return {
        left,
        top,
        above,
        face: next,
        visible: rowInView >= 0 && rowInView < terminal.rows && cell.col >= 0 && cell.col <= terminal.cols,
    };
}

export function resolveTerminalSelectionHandles(host: HTMLElement, terminal: Terminal): Record<number, ReturnType<typeof resolveTerminalHandlePlace>> | null {
    const range = readTerminalSelectionRange(terminal);
    if (!range) {
        return null;
    }
    const end = range.end.col === 0 && range.end.row > 0 ? {col: terminal.cols, row: range.end.row - 1} : range.end;
    return {
        [TERMINAL_SELECT_HANDLE.START]: resolveTerminalHandlePlace(host, terminal, range.start, TERMINAL_HANDLE_FACE.LEFT),
        [TERMINAL_SELECT_HANDLE.END]: resolveTerminalHandlePlace(host, terminal, end, TERMINAL_HANDLE_FACE.RIGHT),
    };
}

export function placeTerminalSelectBar(host: HTMLElement, terminal: Terminal, bar: HTMLElement, point?: TerminalMenuPoint | null): void {
    const layout = readTerminalScreenLayout(host, terminal);
    if (!layout) {
        return;
    }
    let minT: number;
    let maxB: number;
    let minL: number;
    let maxR: number;
    if (point) {
        minT = maxB = point.y;
        minL = maxR = point.x;
    } else {
        const range = readTerminalSelectionRange(terminal);
        if (!range) {
            return;
        }
        const end = exclusiveToInclusive(range.end, terminal.cols);
        minT = layout.rowTop + (range.start.row - layout.viewportY) * layout.cellH;
        maxB = layout.rowTop + (end.row - layout.viewportY + 1) * layout.cellH;
        if (range.start.row === end.row) {
            minL = layout.rowLeft + range.start.col * layout.cellW;
            maxR = layout.rowLeft + (end.col + 1) * layout.cellW;
        } else {
            minL = layout.rowLeft;
            maxR = layout.rowLeft + terminal.cols * layout.cellW;
        }
    }
    let left = (minL + maxR) / 2 - layout.boxRect.left - bar.offsetWidth / 2;
    let top = minT - layout.boxRect.top - bar.offsetHeight - 8;
    if (top < 8) {
        top = maxB - layout.boxRect.top + 8;
    }
    left = Math.max(8, Math.min(left, layout.boxRect.width - bar.offsetWidth - 8));
    top = Math.max(8, Math.min(top, layout.screenRect.bottom - layout.boxRect.top - bar.offsetHeight - 8));
    bar.style.left = `${left}px`;
    bar.style.top = `${top}px`;
}

function resolveTerminalLineHeight(host: HTMLElement, terminal: Terminal): number | null {
    const layout = readTerminalCellMetrics(host, terminal);
    if (!layout) {
        return null;
    }
    return layout.cellH;
}

const pastingTerminals = new WeakSet<Terminal>();

export function terminalIsPasting(terminal: Terminal): boolean {
    return pastingTerminals.has(terminal);
}

export function pasteTerminalText(terminal: Terminal, text: string): void {
    pastingTerminals.add(terminal);
    try {
        terminal.paste(text);
    } finally {
        pastingTerminals.delete(terminal);
    }
}

export function copyTerminalSelection(terminal: Terminal, writeText: (text: string) => Promise<void>,
    onCopied?: () => void, onError?: (error: unknown) => void): boolean {
    if (!terminal.hasSelection()) {
        return false;
    }
    const text = terminal.getSelection();
    const range = JSON.stringify(terminal.getSelectionPosition());
    void (async () => {
        try {
            await writeText(text);
            if (terminal.getSelection() === text && JSON.stringify(terminal.getSelectionPosition()) === range) {
                onCopied?.();
            }
        } catch (error) {
            console.error('[terminal] copy failed:', error);
            onError?.(error);
        }
    })();
    return true;
}

export function pasteClipboardToTerminal(readText: () => Promise<string>, onInput: (data: string) => void): void {
    readText().then(text => {
        if (isEmpty(text)) {
            return;
        }
        onInput(text);
    }).catch(err => console.error('[terminal] paste failed:', err));
}

export function bindTerminalHostPointer(
    host: HTMLElement,
    getTerminal: () => Terminal | null,
    mobile: boolean,
    onPaste: () => void,
    onCopySelection: (terminal: Terminal) => void,
    onPasteMenuChange?: (point: TerminalMenuPoint | null) => void,
    onFontChange?: (size: number, finished: boolean) => void,
): () => void {
    const longPressMs = 400;
    let longPressTimer: number | null = null;
    let pointer: { id: number; touch: boolean; x: number; y: number; lastY: number; acc: number; at: number; mode: typeof TERMINAL_POINTER_MODE[keyof typeof TERMINAL_POINTER_MODE] } | null = null;
    let lastTouch = -Infinity;
    const viewport = terminalTouchViewport(host, getTerminal, () => {
        const terminal = getTerminal();
        return terminal ? resolveTerminalLineHeight(host, terminal) ?? 16 : 16;
    }, onFontChange);
    const clearLongPressTimer = (): void => {
        if (longPressTimer === null) {
            return;
        }
        window.clearTimeout(longPressTimer);
        longPressTimer = null;
    };
    const hideIme = (): void => {
        if (!mobile || readHardKeyboardConnected()) {
            return;
        }
        const terminal = getTerminal();
        if (!terminal) {
            return;
        }
        const textarea = terminal.textarea;
        if (!textarea) {
            console.warn('xterm textarea missing');
            return;
        }
        textarea.setAttribute('inputmode', 'none');
        terminal.blur();
        requestSoftInput(false);
    };
    const showIme = (): void => {
        const terminal = getTerminal();
        if (!terminal) {
            return;
        }
        if (mobile) {
            prepareMobileIme(terminal, readHardKeyboardConnected());
        }
        terminal.focus();
        if (mobile && !readHardKeyboardConnected()) {
            requestSoftInput(true);
        }
    };
    const applyScroll = (clientY: number, time: number): void => {
        if (pointer === null || pointer.mode === TERMINAL_POINTER_MODE.SELECTING) {
            return;
        }
        const terminal = getTerminal();
        if (!terminal) {
            return;
        }
        const dy = clientY - pointer.lastY;
        const lineHeight = resolveTerminalLineHeight(host, terminal);
        if (!lineHeight) {
            return;
        }
        pointer.lastY = clientY;
        viewport.scroll(-dy, pointer.x, clientY, time);
    };
    const dispatch = (kind: typeof TERMINAL_POINTER[keyof typeof TERMINAL_POINTER], event?: Event): void => {
        if (kind === TERMINAL_POINTER.DOWN) {
            if (!(event instanceof PointerEvent)) {
                return;
            }
            if (!mobile && event.button !== 0) {
                if (event.button === 2) {
                    event.preventDefault();
                }
                return;
            }
            if (mobile && event.target instanceof Element) {
                const ignore = event.target.closest('[data-tm-gesture-ignore="true"]');
                if (ignore instanceof HTMLElement && ignore !== host) {
                    return;
                }
            }
            if (mobile) {
                onPasteMenuChange?.(null);
            }
            if (pointer !== null) {
                dispatch(TERMINAL_POINTER.CANCEL);
                return;
            }
            const touch = mobile && event.pointerType !== 'mouse';
            pointer = {id: event.pointerId, touch, x: event.clientX, y: event.clientY, lastY: event.clientY, acc: 0, at: event.timeStamp, mode: TERMINAL_POINTER_MODE.PENDING};
            if (!mobile) {
                return;
            }
            event.preventDefault();
            if (touch) {
                lastTouch = performance.now();
                event.stopPropagation();
                host.setPointerCapture(event.pointerId);
                if (!document.documentElement.classList.contains('tm-keyboard-open')) {
                    getTerminal()?.textarea?.setAttribute('inputmode', 'none');
                }
            } else {
                lastTouch = -Infinity;
            }
            clearLongPressTimer();
            longPressTimer = window.setTimeout(() => {
                longPressTimer = null;
                dispatch(TERMINAL_POINTER.HOLD);
            }, longPressMs);
            return;
        }
        if (kind === TERMINAL_POINTER.HOLD) {
            if (pointer === null || pointer.mode !== TERMINAL_POINTER_MODE.PENDING) {
                return;
            }
            const terminal = getTerminal();
            if (!terminal) {
                return;
            }
            pointer.mode = TERMINAL_POINTER_MODE.SELECTING;
            if (!document.documentElement.classList.contains('tm-keyboard-open')) {
                hideIme();
            }
            const cell = resolveTerminalCell(host, terminal, pointer.x, pointer.y);
            if (!cell) {
                return;
            }
            if (!hasTerminalTextAtCell(terminal, cell)) {
                terminal.clearSelection();
                onPasteMenuChange?.({x: pointer.x, y: pointer.y});
                return;
            }
            expandTerminalSelectToWord(terminal, cell);
            terminal.refresh(0, terminal.rows - 1);
            return;
        }
        if (kind === TERMINAL_POINTER.MOVE) {
            if (pointer === null || pointer.mode === TERMINAL_POINTER_MODE.SELECTING || !mobile) {
                return;
            }
            if (!(event instanceof PointerEvent)) {
                return;
            }
            if (event.pointerId !== pointer.id) {
                return;
            }
            if (pointer.touch) {
                event.stopPropagation();
            }
            const terminal = getTerminal();
            if (!terminal) {
                return;
            }
            const dx = event.clientX - pointer.x;
            const dy = event.clientY - pointer.y;
            const lineHeight = resolveTerminalLineHeight(host, terminal);
            if (!lineHeight) {
                return;
            }
            const slop = Math.min(12, Math.max(8, lineHeight / 2));
            if (pointer.mode === TERMINAL_POINTER_MODE.PENDING && dx * dx + dy * dy < slop * slop) {
                return;
            }
            pointer.mode = TERMINAL_POINTER_MODE.SCROLLING;
            clearLongPressTimer();
            event.preventDefault();
            applyScroll(event.clientY, event.timeStamp);
            return;
        }
        if (kind === TERMINAL_POINTER.UP) {
            if (!(event instanceof PointerEvent)) {
                return;
            }
            if (pointer !== null && event.pointerId !== pointer.id) {
                return;
            }
            if (pointer?.touch) {
                lastTouch = performance.now();
                event.preventDefault();
                event.stopPropagation();
            }
            clearLongPressTimer();
            if (mobile && pointer !== null && pointer.mode === TERMINAL_POINTER_MODE.PENDING && event.timeStamp - pointer.at >= longPressMs) {
                dispatch(TERMINAL_POINTER.HOLD);
            }
            const start = pointer;
            pointer = null;
            if (start?.mode === TERMINAL_POINTER_MODE.SCROLLING) {
                viewport.release(event.timeStamp);
            }
            if (start === null || start.mode !== TERMINAL_POINTER_MODE.PENDING) {
                return;
            }
            const terminal = getTerminal();
            if (!terminal) {
                return;
            }
            const lineHeight = resolveTerminalLineHeight(host, terminal);
            if (!lineHeight) {
                return;
            }
            if (Math.abs(event.clientX - start.x) > lineHeight || Math.abs(event.clientY - start.y) > lineHeight) {
                return;
            }
            if (mobile && terminal.hasSelection()) {
                terminal.clearSelection();
                return;
            }
            showIme();
            return;
        }
        if (kind === TERMINAL_POINTER.CANCEL) {
            clearLongPressTimer();
            pointer = null;
            return;
        }
        if (!(event instanceof MouseEvent)) {
            return;
        }
        event.preventDefault();
        event.stopPropagation();
        if (mobile) {
            clearLongPressTimer();
            dispatch(TERMINAL_POINTER.HOLD);
            return;
        }
        const terminal = getTerminal();
        if (!terminal) {
            return;
        }
        if (terminal.hasSelection()) {
            onCopySelection(terminal);
            return;
        }
        onPaste();
    };
    const onDown = (event: PointerEvent): void => {
        if (mobile && viewport.down(event)) {
            event.preventDefault();
            event.stopPropagation();
            dispatch(TERMINAL_POINTER.CANCEL);
            return;
        }
        dispatch(TERMINAL_POINTER.DOWN, event);
    };
    const onMove = (event: PointerEvent): void => {
        if (mobile && viewport.move(event)) {
            event.preventDefault();
            event.stopPropagation();
            return;
        }
        dispatch(TERMINAL_POINTER.MOVE, event);
    };
    const onUp = (event: PointerEvent): void => {
        if (mobile && viewport.end(event)) {
            event.preventDefault();
            event.stopPropagation();
            dispatch(TERMINAL_POINTER.CANCEL);
            return;
        }
        dispatch(TERMINAL_POINTER.UP, event);
    };
    const onCancel = (event: PointerEvent): void => {
        viewport.cancel();
        dispatch(TERMINAL_POINTER.CANCEL, event);
    };
    const onMenu = (event: Event): void => {
        dispatch(TERMINAL_POINTER.MENU, event);
    };
    const onTouchStart = (event: TouchEvent): void => {
        if (!pointer?.touch && !viewport.active()) {
            return;
        }
        event.preventDefault();
        event.stopPropagation();
    };
    const onCompatibilityMouse = (event: MouseEvent): void => {
        const source = (event as MouseEvent & { sourceCapabilities?: { firesTouchEvents: boolean } }).sourceCapabilities;
        if (!(source?.firesTouchEvents ?? performance.now() - lastTouch < 800)) {
            return;
        }
        event.preventDefault();
        event.stopImmediatePropagation();
    };
    const onBlur = (): void => {
        viewport.cancel();
        dispatch(TERMINAL_POINTER.CANCEL);
    };
    if (mobile) {
        host.style.touchAction = 'none';
        host.addEventListener('touchstart', onTouchStart, {capture: true, passive: false});
        host.addEventListener('mousedown', onCompatibilityMouse, true);
        host.addEventListener('click', onCompatibilityMouse, true);
        window.addEventListener('blur', onBlur);
    }
    host.addEventListener('pointerdown', onDown, true);
    host.addEventListener('pointermove', onMove, {capture: true, passive: false});
    host.addEventListener('pointerup', onUp, true);
    host.addEventListener('pointercancel', onCancel, true);
    host.addEventListener('contextmenu', onMenu, true);
    return () => {
        viewport.cancel();
        clearLongPressTimer();
        host.removeEventListener('pointerdown', onDown, true);
        host.removeEventListener('pointermove', onMove, true);
        host.removeEventListener('pointerup', onUp, true);
        host.removeEventListener('pointercancel', onCancel, true);
        host.removeEventListener('contextmenu', onMenu, true);
        host.removeEventListener('touchstart', onTouchStart, true);
        host.removeEventListener('mousedown', onCompatibilityMouse, true);
        host.removeEventListener('click', onCompatibilityMouse, true);
        window.removeEventListener('blur', onBlur);
        host.style.touchAction = '';
    };
}

function isWindowsPath(value: string): boolean {
    if (value.includes('\\')) {
        return true;
    }
    return /^[A-Za-z]:/.test(value);
}

function isWindowsDropTarget(cwd: string | null, path: string): boolean {
    if (cwd != null && isWindowsPath(cwd)) {
        return true;
    }
    return isWindowsPath(path);
}

export function quoteTerminalDropPath(path: string, cwd: string | null): string {
    if (isWindowsDropTarget(cwd, path)) {
        if (!/[ \t"&()[\]{}^=;!'+,`~]/.test(path)) {
            return path;
        }
        return `"${path.replace(/"/g, '\\"')}"`;
    }
    if (!/[\s'"\\$`&|;<>()[\]{}*?!~#]/.test(path)) {
        return path;
    }
    return `'${path.replace(/'/g, `'\\''`)}'`;
}

export function formatTerminalDropPaths(paths: string[], cwd: string | null): string {
    const quoted: string[] = [];
    for (const path of paths) {
        if (isEmpty(path)) {
            continue;
        }
        quoted.push(quoteTerminalDropPath(path, cwd));
    }
    return quoted.join(' ');
}

export function resolveAccessoryKey(data: string): { key: string; code: string; keyCode: number } | null {
    if (data === '\r' || data === '\n') {
        return {key: 'Enter', code: 'Enter', keyCode: 13};
    }
    if (data === '\t') {
        return {key: 'Tab', code: 'Tab', keyCode: 9};
    }
    if (data === '\x1b') {
        return {key: 'Escape', code: 'Escape', keyCode: 27};
    }
    if (data === '\x1b[A') {
        return {key: 'ArrowUp', code: 'ArrowUp', keyCode: 38};
    }
    if (data === '\x1b[B') {
        return {key: 'ArrowDown', code: 'ArrowDown', keyCode: 40};
    }
    if (data === '\x1b[C') {
        return {key: 'ArrowRight', code: 'ArrowRight', keyCode: 39};
    }
    if (data === '\x1b[D') {
        return {key: 'ArrowLeft', code: 'ArrowLeft', keyCode: 37};
    }
    if (data === '\x1b[H') {
        return {key: 'Home', code: 'Home', keyCode: 36};
    }
    if (data === '\x1b[F') {
        return {key: 'End', code: 'End', keyCode: 35};
    }
    if (data === '\x1b[5~') {
        return {key: 'PageUp', code: 'PageUp', keyCode: 33};
    }
    if (data === '\x1b[6~') {
        return {key: 'PageDown', code: 'PageDown', keyCode: 34};
    }
    return null;
}

export function dispatchAccessoryData(data: string, dispatchKeyEvent: (event: KeyboardEvent) => boolean): boolean {
    const mapped = resolveAccessoryKey(data);
    if (!mapped) {
        return true;
    }
    return dispatchKeyEvent(new KeyboardEvent('keydown', {
        key: mapped.key,
        code: mapped.code,
        keyCode: mapped.keyCode,
        which: mapped.keyCode,
        bubbles: true,
        cancelable: true,
    }));
}

export function parseOsc7Cwd(payload: string): string | null {
    const marker = 'file://';
    if (!payload.startsWith(marker)) {
        return null;
    }
    const rest = payload.slice(marker.length);
    const pathStart = rest.indexOf('/');
    if (pathStart < 0) {
        return null;
    }
    return decodeURIComponent(rest.slice(pathStart));
}

export function resolveTerminalWindowSize(terminal: Terminal, container: HTMLElement): WindowSize {
    return {
        dpi: 0,
        cols: terminal.cols,
        rows: terminal.rows,
        pixel_width: container.clientWidth,
        pixel_height: container.clientHeight,
    };
}

export function readTerminalScreen(terminal: Terminal): TerminalScreen {
    const buffer = terminal.buffer.active;
    const lines: string[] = [];
    for (let row = 0; row < buffer.length; row++) {
        const line = buffer.getLine(row);
        lines.push(line ? line.translateToString(true) : '');
    }
    while (lines.length > 0 && lines[lines.length - 1] === '') {
        lines.pop();
    }
    return {text: lines.join('\n'), cursorRow: buffer.baseY + buffer.cursorY, cursorCol: buffer.cursorX};
}

export function attachTerminalWebgl(terminal: Terminal, onLost: () => void): WebglAddon | null {
    if (window.devicePixelRatio !== 1) {
        return null;
    }
    try {
        const webgl = new WebglAddon();
        webgl.onContextLoss(() => {
            webgl.dispose();
            onLost();
        });
        terminal.loadAddon(webgl);
        return webgl;
    } catch {
        console.warn('[terminal] WebGL unavailable, falling back to canvas/dom renderer');
        return null;
    }
}

export function waitForTerminalStableFit(
    propose: () => { cols: number; rows: number } | undefined | null,
    onReady: (dimsReady: boolean) => void,
    attempt = 0,
): void {
    const proposed = propose();
    const dimsReady = !!proposed && proposed.cols > 0 && proposed.rows > 0;
    const fontsReady = document.fonts ? document.fonts.status === 'loaded' : true;
    if ((dimsReady && fontsReady) || attempt > 60) {
        onReady(dimsReady);
        return;
    }
    requestAnimationFrame(() => waitForTerminalStableFit(propose, onReady, attempt + 1));
}

export function closeTerminal(paneId: string, dragMode: number, data: any, closePane?: (dragMode: number, data: any) => void) {
    if (dragMode == PANE_DRAG_MODE.TAB) {
        const tab = data as TabConfig;
        for (const pane of tab.panes) {
            if (pane.id === paneId) {
                closePane?.(dragMode, data);
                break;
            }
        }
    } else {
        const pane = data as PaneConfig;
        if (pane.id != paneId) {
            return
        }
        closePane?.(dragMode, data);
    }
}
