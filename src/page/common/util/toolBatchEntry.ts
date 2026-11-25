import {isNil, isNotNil} from '@/common/util/StringUtil.ts';

export interface ToolBatchEntry {
    key: string;
    label: string;
    openPath: string | null;
}

function isAbsolute(path: string): boolean {
    return path.startsWith('/') || (path.length > 2 && path[1] === ':');
}

function joinRoot(root: string | null, path: string): string {
    if (isAbsolute(path) || isNil(root) || root.length === 0) {
        return path;
    }
    return `${root.replace(/[/\\]+$/, '')}/${path}`;
}

function readString(source: Record<string, unknown>, key: string): string | null {
    const value = source[key] as string | undefined;
    if (isNil(value) || value.length === 0) {
        return null;
    }
    return value;
}

function readObjectArray(source: Record<string, unknown>, key: string): Record<string, unknown>[] | null {
    const value = source[key];
    if (!Array.isArray(value)) {
        return null;
    }
    return value.filter((item): item is Record<string, unknown> => !isNil(item) && item instanceof Object);
}

export function resolveToolBatchEntries(args: unknown): ToolBatchEntry[] {
    if (isNil(args)) {
        return [];
    }
    const record = args as Record<string, unknown>;
    const root = readString(record, 'root');

    for (const key of ['files', 'edits']) {
        const items = readObjectArray(record, key);
        if (isNil(items) || items.length === 0) {
            continue;
        }
        const entries: ToolBatchEntry[] = [];
        for (let index = 0; index < items.length; index++) {
            const path = readString(items[index], 'path');
            if (isNil(path)) {
                continue;
            }
            const deviceId = readString(items[index], 'device_id') ?? readString(record, 'device_id');
            const label = key === 'files' && isNotNil(deviceId) && isNotNil(readString(record, 'device_id')) ? `${deviceId}:${path}` : path;
            entries.push({key: `${key}-${index}-${label}`, label, openPath: joinRoot(root, path)});
        }
        if (entries.length > 0) {
            return entries;
        }
    }

    for (const key of ['urls', 'paths', 'queries']) {
        const items = record[key];
        if (!Array.isArray(items)) {
            continue;
        }
        const entries: ToolBatchEntry[] = [];
        for (let index = 0; index < items.length; index++) {
            const item = items[index];
            if (isNil(item) || (item as string).length === 0) {
                continue;
            }
            const text = item as string;
            entries.push({key: `${key}-${index}-${text}`, label: text, openPath: key === 'paths' ? joinRoot(root, text) : null});
        }
        if (entries.length > 0) {
            return entries;
        }
    }

    const path = readString(record, 'path');
    if (path !== null) {
        return [{key: `path-${path}`, label: path, openPath: joinRoot(root, path)}];
    }
    const query = readString(record, 'query');
    if (query !== null) {
        return [{key: `query-${query}`, label: query, openPath: null}];
    }
    return [];
}

export function splitToolBatchResult(result: string | null): Map<string, string> {
    const out = new Map<string, string>();
    if (isNil(result) || result.length === 0) {
        return out;
    }
    const lines = result.split('\n');
    let current: string | null = null;
    let buffer: string[] = [];
    const flush = (): void => {
        if (isNil(current)) {
            return;
        }
        out.set(current, buffer.join('\n').trim());
        buffer = [];
    };
    for (const line of lines) {
        const fileHead = /^=====\s+(.*?)(?:\s+\([^)]*\))?\s+=====$/.exec(line);
        if (fileHead) {
            flush();
            current = fileHead[1];
            continue;
        }
        const groupHead = /^##\s+(.*?)(?:\s+\([^)]*\))?$/.exec(line);
        if (groupHead) {
            flush();
            current = groupHead[1];
            continue;
        }
        if (!isNil(current)) {
            buffer.push(line);
        }
    }
    flush();
    return out;
}
