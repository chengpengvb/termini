import {isNil} from "@/common/util/StringUtil.ts";

export function resolveDisplayNumber(n: number | null | undefined): string | undefined {
    if (isNil(n) || n === 0) {
        return undefined;
    }
    return String(n);
}

export function resolveOptionalNumber(n: number | null | undefined): string | undefined {
    if (isNil(n)) {
        return undefined;
    }
    if (n === 0) {
        return undefined;
    }
    return String(n);
}

export function parseOptionalSeconds(raw: string): number | null {
    if (!raw.trim()) {
        return null;
    }
    const parsed = Number(raw);
    if (Number.isFinite(parsed) && parsed > 0) {
        return Math.trunc(parsed);
    }
    return null;
}


export function parsePort(raw: string): number {
    if (!raw.trim()) {
        return 0;
    }
    const parsed = Number(raw);
    if (Number.isFinite(parsed) && parsed > 0) {
        return Math.trunc(parsed);
    }
    return 0;
}

export function clampInt(raw: string, min: number, max: number, fallback: number): number {
    const parsed = Number(raw.trim());
    if (!Number.isFinite(parsed)) {
        return fallback;
    }
    return Math.min(Math.max(Math.trunc(parsed), min), max);
}

export function parseNonNegativeInt(raw: string): number {
    if (!raw.trim()) {
        return 0;
    }
    const parsed = Number(raw);
    if (Number.isFinite(parsed) && parsed >= 0) {
        return Math.trunc(parsed);
    }
    return 0;
}

