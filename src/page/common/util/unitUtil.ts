import {isEmpty} from '@/common/util/StringUtil.ts';
import {JOB_DISPLAY_UNIT} from '@/common/model/JobIndicator.ts';

interface UnitScale {
    label: string;
    factor: number;
}

const BYTES: UnitScale[] = [
    {label: 'B', factor: 1},
    {label: 'KB', factor: 1024},
    {label: 'MB', factor: 1024 ** 2},
    {label: 'GB', factor: 1024 ** 3},
    {label: 'TB', factor: 1024 ** 4},
];

const BITRATE: UnitScale[] = [
    {label: 'bps', factor: 1},
    {label: 'Kbps', factor: 1e3},
    {label: 'Mbps', factor: 1e6},
    {label: 'Gbps', factor: 1e9},
];

const MILLIS: UnitScale[] = [
    {label: 'ms', factor: 1},
    {label: 's', factor: 1000},
    {label: 'min', factor: 60_000},
    {label: 'h', factor: 3600_000},
];

const SECONDS: UnitScale[] = [
    {label: 's', factor: 1},
    {label: 'min', factor: 60},
    {label: 'h', factor: 3600},
    {label: 'd', factor: 86400},
];

const SCALES: Record<number, UnitScale[]> = {
    [JOB_DISPLAY_UNIT.BYTES]: BYTES,
    [JOB_DISPLAY_UNIT.BITRATE]: BITRATE,
    [JOB_DISPLAY_UNIT.MILLIS]: MILLIS,
    [JOB_DISPLAY_UNIT.SECONDS]: SECONDS,
};

export function unitScales(displayUnit: number | null | undefined): UnitScale[] {
    return SCALES[displayUnit ?? JOB_DISPLAY_UNIT.NONE] ?? [];
}

/** 挑最大的、还能让读数不小于 1 的那一档 */
function pickScale(value: number, scales: UnitScale[]): UnitScale {
    let picked = scales[0];
    for (const scale of scales) {
        if (Math.abs(value) >= scale.factor) {
            picked = scale;
        }
    }
    return picked;
}

function trimNumber(value: number): string {
    if (Number.isInteger(value)) {
        return String(value);
    }
    return String(Number(value.toFixed(2)));
}

export function formatDisplay(value: number, displayUnit: number | null | undefined, unit: string): string {
    if (!Number.isFinite(value)) {
        return '—';
    }
    if (displayUnit === JOB_DISPLAY_UNIT.COUNT) {
        return appendUnit(value.toLocaleString('zh-CN', {maximumFractionDigits: 2}), unit);
    }
    const scales = unitScales(displayUnit);
    if (scales.length === 0) {
        return appendUnit(trimNumber(value), unit);
    }
    const scale = pickScale(value, scales);
    return `${trimNumber(value / scale.factor)} ${scale.label}`;
}

export function formatDisplayText(raw: string | null | undefined, displayUnit: number | null | undefined, unit: string): string {
    if (isEmpty(raw)) {
        return '—';
    }
    const value = Number(raw);
    if (!Number.isFinite(value)) {
        return appendUnit(raw, unit);
    }
    return formatDisplay(value, displayUnit, unit);
}

export function splitScaled(raw: string | null | undefined, displayUnit: number | null | undefined): {text: string; factor: number} {
    const scales = unitScales(displayUnit);
    const value = Number(raw);
    if (scales.length === 0 || !Number.isFinite(value) || value === 0) {
        return {text: raw ?? '', factor: 1};
    }
    const scale = pickScale(value, scales);
    return {text: trimNumber(value / scale.factor), factor: scale.factor};
}

export function toBaseText(text: string, factor: number): string {
    const value = Number(text);
    if (!Number.isFinite(value)) {
        return text;
    }
    return String(Math.round(value * factor * 1000) / 1000);
}

function appendUnit(text: string, unit: string): string {
    return isEmpty(unit) ? text : `${text} ${unit}`;
}
