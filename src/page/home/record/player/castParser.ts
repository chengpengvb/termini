export const CAST_OUTPUT = 'o';
export const CAST_MARKER = 'm';

export interface CastOutput {
    at: number;
    data: string;
}

export interface CastData {
    total: number;
    outputs: CastOutput[];
    activity: number[];
}

const ACTIVITY_BUCKETS = 120;
const MARKER_WEIGHT = 24;

function buildActivity(outputs: CastOutput[], markers: number[], total: number): number[] {
    const raw = new Array<number>(ACTIVITY_BUCKETS).fill(0);
    if (total <= 0) {
        return raw;
    }
    const bucketOf = (at: number): number => Math.min(ACTIVITY_BUCKETS - 1, Math.max(0, Math.floor(at / total * ACTIVITY_BUCKETS)));
    for (const output of outputs) {
        raw[bucketOf(output.at)] += Math.log2(1 + output.data.length);
    }
    for (const at of markers) {
        raw[bucketOf(at)] += MARKER_WEIGHT;
    }
    const peak = raw.reduce((max, value) => Math.max(max, value), 0);
    if (peak <= 0) {
        return raw;
    }
    return raw.map(value => Math.sqrt(value / peak));
}

export function parseCast(content: string): CastData {
    const lines = content.split('\n');
    const outputs: CastOutput[] = [];
    const markers: number[] = [];
    let last = 0;
    for (let i = 1; i < lines.length; i += 1) {
        const line = lines[i].trim();
        if (line.length === 0) {
            continue;
        }
        const [at, kind, data] = JSON.parse(line) as [number, string, string];
        last = Math.max(last, at);
        if (kind === CAST_OUTPUT) {
            outputs.push({at, data});
        } else if (kind === CAST_MARKER) {
            markers.push(at);
        }
    }
    return {total: last, outputs, activity: buildActivity(outputs, markers, last)};
}
