import type {ITheme, Terminal} from '@xterm/xterm';

interface Rect {left: number; top: number; right: number; bottom: number}
export interface MagnifierTarget {col: number; row: number; fingerX: number; fingerY: number}

const ANSI_KEYS = ['black', 'red', 'green', 'yellow', 'blue', 'magenta', 'cyan', 'white',
    'brightBlack', 'brightRed', 'brightGreen', 'brightYellow', 'brightBlue', 'brightMagenta', 'brightCyan', 'brightWhite'] as const;
const ANSI_DEFAULT = ['#2e3436', '#cc0000', '#4e9a06', '#c4a000', '#3465a4', '#75507b', '#06989a', '#d3d7cf',
    '#555753', '#ef2929', '#8ae234', '#fce94f', '#729fcf', '#ad7fa8', '#34e2e2', '#eeeeec'];
const clamp = (value: number, min: number, max: number): number => Math.max(min, Math.min(max, value));

export function placeTerminalMagnifier(screen: Rect, viewport: Rect, x: number, y: number, fingerX: number, fingerY: number) {
    const left = Math.max(screen.left, viewport.left) + 8;
    const right = Math.min(screen.right, viewport.right) - 8;
    const top = Math.max(screen.top, viewport.top) + 8;
    const bottom = Math.min(screen.bottom, viewport.bottom) - 8;
    const width = Math.min(180, right - left);
    const height = Math.min(76, bottom - top);
    if (width < 60 || height < 32) {
        return null;
    }
    let nextX = x - width / 2;
    let nextY = Math.min(y, fingerY) - height - 28;
    if (nextY < top) {
        const below = Math.max(y, fingerY) + 32;
        if (below + height <= bottom) {
            nextY = below;
        } else {
            nextX = fingerX + 36 + width <= right ? fingerX + 36 : fingerX - width - 36;
            nextY = y - height / 2;
        }
    }
    return {left: clamp(nextX, left, right - width), top: clamp(nextY, top, bottom - height), width, height};
}

function paletteColor(theme: ITheme, index: number): string {
    if (index < 16) {
        return theme[ANSI_KEYS[index]] ?? ANSI_DEFAULT[index];
    }
    if (theme.extendedAnsi?.[index - 16]) {
        return theme.extendedAnsi[index - 16];
    }
    if (index >= 232) {
        const value = 8 + (index - 232) * 10;
        return 'rgb(' + [value, value, value].join(',') + ')';
    }
    const value = index - 16;
    const levels = [0, 95, 135, 175, 215, 255];
    return 'rgb(' + [levels[Math.floor(value / 36)], levels[Math.floor(value / 6) % 6], levels[value % 6]].join(',') + ')';
}

export function paintTerminalMagnifier(canvas: HTMLCanvasElement, terminal: Terminal, target: MagnifierTarget,
    cellW: number, cellH: number, width: number, height: number): void {
    const context = canvas.getContext('2d');
    if (!context) {
        throw new Error('Terminal magnifier canvas unavailable');
    }
    const ratio = window.devicePixelRatio || 1;
    const zoom = 1.7;
    canvas.width = Math.round(width * ratio);
    canvas.height = Math.round(height * ratio);
    const theme = terminal.options.theme ?? {};
    const bg = theme.background ?? '#000000';
    const fg = theme.foreground ?? '#ffffff';
    context.fillStyle = bg;
    context.fillRect(0, 0, canvas.width, canvas.height);
    const sourceX = clamp((target.col + 0.5) * cellW - width / zoom / 2, 0, Math.max(0, terminal.cols * cellW - width / zoom));
    const sourceY = clamp((target.row - terminal.buffer.active.viewportY + 0.5) * cellH - height / zoom / 2,
        0, Math.max(0, terminal.rows * cellH - height / zoom));
    context.setTransform(ratio * zoom, 0, 0, ratio * zoom, -sourceX * ratio * zoom, -sourceY * ratio * zoom);
    context.textBaseline = 'alphabetic';
    const selection = terminal.getSelectionPosition();
    const selectedStart = selection ? selection.start.y * terminal.cols + selection.start.x : -1;
    const selectedEnd = selection ? selection.end.y * terminal.cols + selection.end.x : -1;
    const firstCol = Math.max(0, Math.floor(sourceX / cellW) - 1);
    const lastCol = Math.min(terminal.cols, Math.ceil((sourceX + width / zoom) / cellW));
    const firstRow = Math.floor(sourceY / cellH);
    const lastRow = Math.min(terminal.rows, Math.ceil((sourceY + height / zoom) / cellH));
    const size = terminal.options.fontSize ?? 15;
    for (let row = firstRow; row < lastRow; row++) {
        const bufferRow = row + terminal.buffer.active.viewportY;
        const line = terminal.buffer.active.getLine(bufferRow);
        for (let col = firstCol; col < lastCol; col++) {
            const cell = line?.getCell(col);
            if (!cell || cell.getWidth() === 0) {
                continue;
            }
            const color = (rgb: boolean, palette: boolean, value: number, fallback: string): string =>
                rgb ? '#' + value.toString(16).padStart(6, '0') : palette ? paletteColor(theme, value) : fallback;
            let fgColor = cell.getFgColor();
            if (cell.isFgPalette() && fgColor < 8 && cell.isBold() && terminal.options.drawBoldTextInBrightColors) {
                fgColor += 8;
            }
            let foreground = color(cell.isFgRGB(), cell.isFgPalette(), fgColor, fg);
            let background = color(cell.isBgRGB(), cell.isBgPalette(), cell.getBgColor(), bg);
            if (cell.isInverse()) {
                [foreground, background] = [background, foreground];
            }
            const x = col * cellW;
            const y = row * cellH;
            const w = cellW * cell.getWidth();
            context.fillStyle = background;
            context.fillRect(x, y, w, cellH);
            const index = bufferRow * terminal.cols + col;
            if (index < selectedEnd && index + cell.getWidth() > selectedStart) {
                context.fillStyle = theme.selectionBackground ?? 'rgba(100,140,220,0.45)';
                context.fillRect(x, y, w, cellH);
                foreground = theme.selectionForeground ?? foreground;
            }
            if (cell.isInvisible()) {
                continue;
            }
            context.font = (cell.isItalic() ? 'italic ' : '') + (cell.isBold() ? terminal.options.fontWeightBold ?? 'bold' : terminal.options.fontWeight ?? 'normal') +
                ' ' + size + 'px ' + (terminal.options.fontFamily ?? 'monospace');
            const metrics = context.measureText(cell.getChars());
            const ascent = metrics.fontBoundingBoxAscent || size * 0.8;
            const descent = metrics.fontBoundingBoxDescent || size * 0.2;
            const baseline = y + (cellH - ascent - descent) / 2 + ascent;
            context.fillStyle = foreground;
            context.globalAlpha = cell.isDim() ? 0.5 : 1;
            context.fillText(cell.getChars(), x, baseline);
            if (cell.isUnderline()) {
                context.fillRect(x, baseline + 1, w, 1);
            }
            if (cell.isStrikethrough()) {
                context.fillRect(x, y + cellH / 2, w, 1);
            }
            context.globalAlpha = 1;
        }
    }
}
