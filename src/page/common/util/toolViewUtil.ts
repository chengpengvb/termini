import {resolveAiToolLabel} from '@/common/const/AiToolConst.ts';
import {isNil, isNotEmpty, isNotNil} from '@/common/util/StringUtil.ts';
import type {RespAiSshBatchItem} from '@/common/model/resp/RespAiSshBatch.ts';
import type {ReplaceConflictView, ReplaceFileGroup} from '@/page/common/dto/agent/ToolView.ts';

export function groupReplaceByFile(list: ReplaceConflictView[]): ReplaceFileGroup[] {
    const groups: ReplaceFileGroup[] = [];
    for (const item of list) {
        const key = item.fullPath ?? item.path;
        const found = groups.find(group => group.key === key);
        if (isNotNil(found)) {
            found.edits.push(item);
            continue;
        }
        groups.push({key, path: item.path, displayName: item.displayName, fullPath: item.fullPath, edits: [item]});
    }
    return groups;
}

export function summarizeToolArgs(args: unknown): string {
    if (isNil(args)) {
        return '';
    }
    if (!(args instanceof Object)) {
        return `${args}`;
    }
    const record = args as Record<string, unknown>;
    const parts: string[] = [];
    for (const key of ['title', 'path', 'root', 'cwd', 'command', 'query', 'name']) {
        const value = record[key];
        if (isNil(value) || value instanceof Object) {
            continue;
        }
        const text = `${value}`;
        if (text.length > 0) {
            parts.push(`${key}=${text}`);
        }
    }
    for (const key of ['files', 'queries', 'edits', 'actions', 'commands']) {
        const value = record[key];
        if (Array.isArray(value) && value.length > 0) {
            parts.push(`${key}×${value.length}`);
        }
    }
    return parts.length > 0 ? parts.join(' ') : JSON.stringify(args);
}

export function resolveToolStepTitle(toolId: string, args: unknown): string {
    const title = (args as {title: string}).title;
    if (isNotEmpty(title)) {
        return title;
    }
    return resolveAiToolLabel(toolId);
}

export function parseDeviceBatchItems(result: string | null): RespAiSshBatchItem[] | null {
    if (isNil(result) || result.length === 0) {
        return null;
    }
    try {
        const parsed: unknown = JSON.parse(result);
        if (!Array.isArray(parsed)) {
            return null;
        }
        return parsed.filter((row): row is RespAiSshBatchItem => !isNil(row) && isNotNil((row as RespAiSshBatchItem).device_id));
    } catch {
        return null;
    }
}
