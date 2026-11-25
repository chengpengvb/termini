export interface ChartSample {
    t: number;
    v: number;
}

export interface ChartPadding {
    top: number;
    left: number;
    right: number;
    bottom: number;
}

export interface HistoryPoint {
    x: number;
    y: number;
}

export interface HistoryGrid {
    y: number;
    value: number;
    label: string;
}

export interface HistoryGeometry {
    line: string;
    area: string;
    baseY: number;
    endX: number;
    startX: number;
    dotX: number;
    dotY: number;
    points: HistoryPoint[];
    grids: HistoryGrid[];
    thresholdY: number | null;
}

export const CHART_SAMPLE_MIN = 1;

export const CHART_RANGES = [
    {key: '1h', label: '1h', durationMs: 3600_000},
    {key: '6h', label: '6h', durationMs: 6 * 3600_000},
    {key: '24h', label: '24h', durationMs: 24 * 3600_000},
] as const;

export type ChartRangeKey = typeof CHART_RANGES[number]['key'];

function spread(samples: ChartSample[]): ChartSample[] {
    if (samples.length > 1) {
        return samples;
    }
    return [...samples, ...samples];
}

export function resolveChartRange(key: string): typeof CHART_RANGES[number] {
    return CHART_RANGES.find(item => item.key === key) ?? CHART_RANGES[0];
}

export function toChartNumber(raw: string | null | undefined): number {
    if (raw == null || raw === '') {
        return Number.NaN;
    }
    const value = Number(raw);
    return Number.isFinite(value) ? value : Number.NaN;
}

export function formatChartTick(millis: number, durationMs: number): string {
    const date = new Date(millis);
    const hours = String(date.getHours()).padStart(2, '0');
    const minutes = String(date.getMinutes()).padStart(2, '0');
    if (durationMs >= 24 * 3600_000) {
        return `${date.getMonth() + 1}/${date.getDate()} ${hours}:${minutes}`;
    }
    return `${hours}:${minutes}`;
}

export function resolveHoverIndex(ratio: number, count: number, width: number, pad: ChartPadding): number | null {
    if (count <= 0 || width <= 0) {
        return null;
    }
    const inner = width - pad.left - pad.right;
    if (inner <= 0) {
        return null;
    }
    const x = ratio * width;
    const clamped = Math.min(Math.max(x, pad.left), width - pad.right);
    const index = Math.round(((clamped - pad.left) / inner) * (count - 1));
    if (index < 0 || index >= count) {
        return null;
    }
    return index;
}

export function buildSparklinePoints(samples: ChartSample[], width: number, height: number, pad: number): string {
    const drawn = spread(samples);
    const values = drawn.map(sample => sample.v);
    const min = Math.min(...values);
    const span = Math.max(...values) - min || 1;
    return drawn.map((sample, index) => {
        const x = pad + (index / (drawn.length - 1)) * (width - pad * 2);
        const y = height - pad - ((sample.v - min) / span) * (height - pad * 2);
        return `${x},${y}`;
    }).join(' ');
}

export function buildHistoryGeometry(samples: ChartSample[], threshold: number, rangeMin: number, rangeMax: number, width: number, height: number, pad: ChartPadding): HistoryGeometry {
    const drawn = spread(samples);
    const values = drawn.map(sample => sample.v);
    let min = Math.min(...values);
    let max = Math.max(...values);
    if (Number.isFinite(threshold)) {
        min = Math.min(min, threshold);
        max = Math.max(max, threshold);
    }
    if (Number.isFinite(rangeMin)) {
        min = Math.min(min, rangeMin);
    }
    if (Number.isFinite(rangeMax)) {
        max = Math.max(max, rangeMax);
    }
    const span = max - min || 1;
    const startX = pad.left;
    const endX = width - pad.right;
    const topY = pad.top;
    const baseY = height - pad.bottom;
    const innerW = endX - startX;
    const innerH = baseY - topY;
    const points = drawn.map((sample, index) => {
        const x = startX + (index / (drawn.length - 1)) * innerW;
        const y = topY + (1 - (sample.v - min) / span) * innerH;
        return {x, y};
    });
    const line = points.map(point => `${point.x},${point.y}`).join(' ');
    const area = `${startX},${baseY} ${line} ${endX},${baseY}`;
    const last = points[points.length - 1];
    const grids: HistoryGrid[] = [];
    for (let step = 0; step <= 2; step++) {
        const ratio = step / 2;
        const value = max - span * ratio;
        const y = topY + ratio * innerH;
        grids.push({y, value, label: trimAxis(value)});
    }
    const thresholdY = Number.isFinite(threshold) ? topY + (1 - (threshold - min) / span) * innerH : null;
    return {
        line,
        area,
        baseY,
        endX,
        startX,
        points,
        grids,
        thresholdY,
        dotX: last.x,
        dotY: last.y,
    };
}

function trimAxis(value: number): string {
    if (Math.abs(value) >= 1000) {
        return value.toLocaleString('zh-CN', {maximumFractionDigits: 0});
    }
    return String(Number(value.toFixed(2)));
}
