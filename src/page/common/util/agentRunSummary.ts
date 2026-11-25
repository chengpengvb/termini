import type {AiChatItem} from '@/common/model/AiChatItem.ts';
import {translate, translateFormat} from '@/common/i18n/i18n.ts';
import {isNil, isNotEmpty, isNotNil} from '@/common/util/StringUtil.ts';
import {RESP_EVENT_AGENT} from '@/common/model/resp/event/RespEventAgent.ts';
import {AGENT_ANSWER_CODE, isAgentAnswerAllowed} from '@/common/model/req/event/WsAgent.ts';
import {AI_TOOL_ID, resolveAiToolId, SFTP_TRANSFER_DIRECTION} from '@/common/const/AiToolConst.ts';

interface SummaryArgsRaw {
    path?: string;
    url?: string;
    command?: string;
    keyword?: string;
    device_id?: string;
    local_path?: string;
    remote_path?: string;
    direction?: number;
    device_ids?: string[];
    root?: string;
    old_string?: string;
    new_string?: string;
    from?: string;
    to?: string;
    edits?: SummaryArgsRaw[];
    commands?: SummaryArgsRaw[];
}

interface RunSummaryEntry {
    callId: string;
    toolName: string;
    args: SummaryArgsRaw | null;
    hasConflict: boolean;
    answerCode: number | null;
    result: string | null;
}

export interface RunSummaryLine {
    tool: string;
    killed: boolean;
    callId: string;
    result: string | null;
    toolId: number | null;
    confirm: number | null;
    target: string | null;
    exitCode: number | null;
    targetFull: string | null;
    resultFull: string | null;
    addCount: number | null;
    delCount: number | null;
}

export const RUN_SUMMARY_CONFIRM = {
    PENDING: 0,
    ALLOW: 1,
    REJECT: 2,
} as const;

const TOOL_I18N: Record<number, string> = {
    [AI_TOOL_ID.LOCAL_SHELL]: 'common.agent.tool.localShell',
    [AI_TOOL_ID.SSH_SHELL_BATCH]: 'common.agent.tool.sshShell',
    [AI_TOOL_ID.TELNET_SHELL_BATCH]: 'common.agent.tool.telnetShell',
    [AI_TOOL_ID.SFTP_TRANSFER_BATCH]: 'common.agent.tool.sftpTransfer',
    [AI_TOOL_ID.FTP_TRANSFER_BATCH]: 'common.agent.tool.ftpTransfer',
    [AI_TOOL_ID.SFTP_FS]: 'common.agent.tool.sftpFs',
    [AI_TOOL_ID.FTP_FS]: 'common.agent.tool.ftpFs',
    [AI_TOOL_ID.DEVICE_LIST]: 'common.agent.tool.deviceList',
    [AI_TOOL_ID.DEVICE_IMPORT]: 'common.agent.tool.deviceImport',
    [AI_TOOL_ID.GROUP_LIST]: 'common.agent.tool.groupList',
    [AI_TOOL_ID.SKILL_LIST]: 'common.agent.tool.skillList',
    [AI_TOOL_ID.FILE_READ]: 'common.agent.tool.fileRead',
    [AI_TOOL_ID.SEARCH_REPLACE]: 'common.agent.tool.searchReplace',
    [AI_TOOL_ID.WEB_FETCH]: 'common.agent.tool.webFetch',
    [AI_TOOL_ID.SFTP_FILE_READ]: 'common.agent.tool.sftpFileRead',
    [AI_TOOL_ID.SFTP_SEARCH_REPLACE]: 'common.agent.tool.sftpSearchReplace',
    [AI_TOOL_ID.PDF_READ]: 'common.agent.tool.pdfRead',
    [AI_TOOL_ID.XLSX_READ]: 'common.agent.tool.xlsxRead',
    [AI_TOOL_ID.XLSX_WRITE]: 'common.agent.tool.xlsxWrite',
    [AI_TOOL_ID.DOCX_READ]: 'common.agent.tool.docxRead',
    [AI_TOOL_ID.DOCX_WRITE]: 'common.agent.tool.docxWrite',
    [AI_TOOL_ID.PDF_WRITE]: 'common.agent.tool.pdfWrite',
    [AI_TOOL_ID.TODO_WRITE]: 'common.agent.tool.todoWrite',
    [AI_TOOL_ID.TASK]: 'common.agent.tool.task',
    [AI_TOOL_ID.SKILL_CREATE]: 'common.agent.tool.skillCreate',
    [AI_TOOL_ID.SKILL_UPDATE]: 'common.agent.tool.skillUpdate',
    [AI_TOOL_ID.MCP_LIST]: 'common.agent.tool.mcpList',
    [AI_TOOL_ID.MCP_CREATE]: 'common.agent.tool.mcpCreate',
    [AI_TOOL_ID.MCP_UPDATE]: 'common.agent.tool.mcpUpdate',
    [AI_TOOL_ID.MCP_DELETE]: 'common.agent.tool.mcpDelete',
    [AI_TOOL_ID.LOG_LIST]: 'common.agent.tool.logList',
    [AI_TOOL_ID.DEVICE_PROFILE_UPDATE]: 'common.agent.tool.deviceProfileUpdate',
    [AI_TOOL_ID.MAIL_SEND]: 'common.agent.tool.mailSend',
    [AI_TOOL_ID.ENTITY_LIST]: 'common.agent.tool.entityList',
    [AI_TOOL_ID.ENTITY_DESCRIBE]: 'common.agent.tool.entityDescribe',
    [AI_TOOL_ID.ENTITY_CALL]: 'common.agent.tool.entityCall',
};

const RESULT_MAX_LEN = 96;
const EXIT_CODE_PATTERN = /(?:^|\n)exit_code=(-?\d+)\s*$/;
const KILLED_PATTERN = /(?:^|\n)exit_code=killed\b/;

function isSummaryEvent(type: number): boolean {
    return type === RESP_EVENT_AGENT.ON_TOOL_CALL
        || type === RESP_EVENT_AGENT.ON_TOOL_CALL_DONE
        || type === RESP_EVENT_AGENT.ON_CONFLICT
        || type === RESP_EVENT_AGENT.ON_CONFLICT_ANSWERED;
}

function resolveToolLabel(toolName: string): string {
    const toolId = resolveAiToolId(toolName);
    if (isNil(toolId)) {
        return toolName;
    }
    const key = TOOL_I18N[toolId];
    if (!key) {
        return toolName;
    }
    return translate(key);
}

function resolveSftpTransferTarget(args: SummaryArgsRaw): string | null {
    const deviceIds = args.device_ids ?? [];
    if (deviceIds.length === 0 || isNil(args.direction)) {
        return null;
    }
    const device = deviceIds.length === 1 ? deviceIds[0] : translateFormat('common.agent.batchDevices', {count: deviceIds.length});
    if (args.direction === SFTP_TRANSFER_DIRECTION.UPLOAD) {
        if (!args.local_path) {
            return null;
        }
        if (!args.remote_path) {
            return translateFormat('common.agent.uploadToDevice', {local: args.local_path, device});
        }
        return translateFormat('common.agent.uploadToPath', {local: args.local_path, device, remote: args.remote_path});
    }
    if (args.direction === SFTP_TRANSFER_DIRECTION.DOWNLOAD) {
        if (!args.remote_path) {
            return null;
        }
        if (!args.local_path) {
            return translateFormat('common.agent.downloadFromDevice', {device, remote: args.remote_path});
        }
        return translateFormat('common.agent.downloadToLocal', {device, remote: args.remote_path, local: args.local_path});
    }
    return null;
}

function resolveDeviceBatchTarget(args: SummaryArgsRaw): string | null {
    const deviceIds = args.device_ids ?? [];
    if (deviceIds.length === 0) {
        return null;
    }
    if (deviceIds.length === 1) {
        return args.command ? `${deviceIds[0]} · ${truncateText(args.command, 48)}` : deviceIds[0];
    }
    if (args.command) {
        return translateFormat('common.agent.batchWithCommand', {count: deviceIds.length, command: truncateText(args.command, 48)});
    }
    return translateFormat('common.agent.batchDevices', {count: deviceIds.length});
}

function resolveRemotePathTarget(args: SummaryArgsRaw): string | null {
    if (!args.device_id) {
        return null;
    }
    if (args.from && args.to) {
        return `${args.device_id} · ${args.from} -> ${args.to}`;
    }
    if (args.path) {
        return `${args.device_id} · ${args.path}`;
    }
    if (args.remote_path) {
        return `${args.device_id} · ${args.remote_path}`;
    }
    if (args.command) {
        return `${args.device_id} · ${truncateText(args.command, 48)}`;
    }
    return args.device_id;
}

function resolveTarget(toolId: number | null, args: SummaryArgsRaw | null): string | null {
    if (!args) {
        return null;
    }
    if (toolId === AI_TOOL_ID.SFTP_TRANSFER_BATCH || toolId === AI_TOOL_ID.FTP_TRANSFER_BATCH) {
        return resolveSftpTransferTarget(args);
    }
    if (toolId === AI_TOOL_ID.SSH_SHELL_BATCH || toolId === AI_TOOL_ID.TELNET_SHELL_BATCH) {
        return resolveDeviceBatchTarget(args);
    }
    if (toolId === AI_TOOL_ID.DEVICE_LIST || toolId === AI_TOOL_ID.GROUP_LIST || toolId === AI_TOOL_ID.SKILL_LIST) {
        if (!args.keyword) {
            return translate('common.agent.all');
        }
        return translateFormat('common.agent.keyword', {text: truncateText(args.keyword, 48)});
    }
    if (toolId === AI_TOOL_ID.DEVICE_PROFILE_UPDATE) {
        if (!args.device_id) {
            return null;
        }
        return args.device_id;
    }
    if (toolId === AI_TOOL_ID.WEB_FETCH) {
        if (!args.url) {
            return null;
        }
        return args.url;
    }
    if (toolId === AI_TOOL_ID.SFTP_FILE_READ || toolId === AI_TOOL_ID.SFTP_SEARCH_REPLACE || toolId === AI_TOOL_ID.SFTP_FS || toolId === AI_TOOL_ID.FTP_FS) {
        return resolveRemotePathTarget(args);
    }
    if (args.path) {
        return args.path;
    }
    if (args.command) {
        return truncateText(args.command, 64);
    }
    const batch = resolveCommandBatch(args);
    if (batch) {
        const head = truncateText(batch[0], 48);
        return batch.length > 1 ? `${head} +${batch.length - 1}` : head;
    }
    if (args.local_path) {
        return args.local_path;
    }
    return null;
}

function resolveCommandBatch(args: SummaryArgsRaw): string[] | null {
    const commands = args.commands ?? [];
    const texts = commands.map(item => item.command).filter((text): text is string => isNotEmpty(text));
    if (texts.length === 0) {
        return null;
    }
    return texts;
}

function resolveTargetFull(args: SummaryArgsRaw | null): string | null {
    if (!args) {
        return null;
    }
    const batch = resolveCommandBatch(args);
    if (batch) {
        return batch.join('\n');
    }
    const rename = args.from && args.to ? `${args.from} -> ${args.to}` : undefined;
    const value = args.command ?? args.path ?? rename ?? args.url ?? args.keyword ?? args.remote_path ?? args.local_path;
    if (!value) {
        return null;
    }
    return value;
}

function resolveConfirm(answerCode: number | null, hasConflict: boolean): number | null {
    if (!hasConflict) {
        return null;
    }
    if (isNil(answerCode)) {
        return RUN_SUMMARY_CONFIRM.PENDING;
    }
    if (isAgentAnswerAllowed(answerCode)) {
        return RUN_SUMMARY_CONFIRM.ALLOW;
    }
    if (answerCode === AGENT_ANSWER_CODE.REJECT) {
        return RUN_SUMMARY_CONFIRM.REJECT;
    }
    return RUN_SUMMARY_CONFIRM.PENDING;
}

function truncateText(text: string, maxLen: number): string {
    const trimmed = text.trim().replace(/\s+/g, ' ');
    if (trimmed.length <= maxLen) {
        return trimmed;
    }
    return `${trimmed.slice(0, maxLen)}…`;
}

function resolveExitCode(result: string): number | null {
    const matched = EXIT_CODE_PATTERN.exec(result);
    if (!matched) {
        return null;
    }
    return Number(matched[1]);
}

function resolveJsonArrayCount(result: string): number | null {
    try {
        const parsed: unknown = JSON.parse(result);
        if (!Array.isArray(parsed)) {
            return null;
        }
        return parsed.length;
    } catch {
        return null;
    }
}

interface BatchResultItem {
    ok?: boolean;
}

function resolveBatchResultSummary(result: string): string | null {
    try {
        const parsed: unknown = JSON.parse(result);
        if (!Array.isArray(parsed)) {
            return null;
        }
        let okCount = 0;
        let failCount = 0;
        for (const item of parsed as BatchResultItem[]) {
            if (!item) {
                failCount += 1;
                continue;
            }
            if (item.ok === true) {
                okCount += 1;
                continue;
            }
            failCount += 1;
        }
        return translateFormat('common.agent.successFail', {ok: okCount, fail: failCount});
    } catch {
        return null;
    }
}

function resolveListResultSummary(result: string, unit: string): string | null {
    const count = resolveJsonArrayCount(result);
    if (isNil(count)) {
        return null;
    }
    return `${count} ${unit}`;
}

function resolveReadResultSummary(result: string): string | null {
    const trimmed = result.trim();
    if (!trimmed) {
        return translate('common.agent.emptyContent');
    }
    const lineCount = trimmed.split('\n').length;
    if (lineCount <= 1) {
        return truncateText(trimmed, RESULT_MAX_LEN);
    }
    return translateFormat('common.agent.linesPreview', {lines: lineCount, preview: truncateText(trimmed, 48)});
}

function resolveResultBody(toolId: number | null, result: string | null): string | null {
    if (!result) {
        return null;
    }
    if (toolId === AI_TOOL_ID.SSH_SHELL_BATCH || toolId === AI_TOOL_ID.TELNET_SHELL_BATCH || toolId === AI_TOOL_ID.SFTP_TRANSFER_BATCH || toolId === AI_TOOL_ID.FTP_TRANSFER_BATCH) {
        return resolveBatchResultSummary(result);
    }
    if (toolId === AI_TOOL_ID.DEVICE_LIST) {
        return resolveListResultSummary(result, translate('common.agent.deviceUnit'));
    }
    if (toolId === AI_TOOL_ID.GROUP_LIST) {
        return resolveListResultSummary(result, translate('common.agent.groupUnit'));
    }
    if (toolId === AI_TOOL_ID.FILE_READ || toolId === AI_TOOL_ID.SFTP_FILE_READ) {
        return resolveReadResultSummary(result);
    }
    const withoutExit = result.replace(EXIT_CODE_PATTERN, '').trim();
    if (!withoutExit) {
        return null;
    }
    return truncateText(withoutExit, RESULT_MAX_LEN);
}

function countHunkLines(text: string | undefined): number {
    if (!text) {
        return 0;
    }
    return text.split('\n').length;
}

function isEditTool(toolId: number | null): boolean {
    return toolId === AI_TOOL_ID.SEARCH_REPLACE || toolId === AI_TOOL_ID.SFTP_SEARCH_REPLACE;
}

function resolveEditFullPath(root: string | undefined, path: string): string {
    if (path.startsWith('/') || (path.length > 2 && path[1] === ':')) {
        return path;
    }
    if (!root) {
        return path;
    }
    return `${root.replace(/[/\\]+$/, '')}/${path}`;
}

function buildSummaryLine(entry: RunSummaryEntry): RunSummaryLine {
    const toolId = resolveAiToolId(entry.toolName);
    const exitCode = entry.result ? resolveExitCode(entry.result) : null;
    return {
        killed: entry.result ? KILLED_PATTERN.test(entry.result) : false,
        callId: entry.callId,
        tool: resolveToolLabel(entry.toolName),
        toolId,
        target: resolveTarget(toolId, entry.args),
        targetFull: resolveTargetFull(entry.args),
        confirm: resolveConfirm(entry.answerCode, entry.hasConflict),
        result: resolveResultBody(toolId, entry.result),
        resultFull: entry.result,
        exitCode,
        addCount: null,
        delCount: null,
    };
}

function expandEditLines(entry: RunSummaryEntry, base: RunSummaryLine): RunSummaryLine[] {
    const args = entry.args;
    if (!args) {
        return [base];
    }
    const hunks = Array.isArray(args.edits) && args.edits.length > 0 ? args.edits : [args];
    const byPath = new Map<string, {path: string; add: number; del: number}>();
    for (const hunk of hunks) {
        if (!hunk.path) {
            continue;
        }
        const fullPath = resolveEditFullPath(args.root, hunk.path);
        const device = hunk.device_id ?? args.device_id;
        const key = device ? `${device}:${fullPath}` : fullPath;
        const current = byPath.get(key);
        const add = countHunkLines(hunk.new_string);
        const del = countHunkLines(hunk.old_string);
        if (current) {
            current.add += add;
            current.del += del;
            continue;
        }
        byPath.set(key, {path: hunk.path, add, del});
    }
    if (byPath.size === 0) {
        return [base];
    }
    const rows: RunSummaryLine[] = [];
    for (const [fullPath, stat] of byPath) {
        rows.push({
            ...base,
            target: stat.path,
            targetFull: fullPath,
            result: null,
            addCount: stat.add,
            delCount: stat.del,
        });
    }
    return rows;
}

export class AgentRunSummaryBuffer {
    private readonly entriesByReq = new Map<string, Map<string, RunSummaryEntry>>();

    clear(): void {
        this.entriesByReq.clear();
    }

    accumulate(item: AiChatItem): boolean {
        if (!isSummaryEvent(item.type)) {
            return false;
        }
        const reqId = item.context.reqId;
        if (!reqId) {
            return false;
        }
        const callId = item.context.callId;
        if (!callId) {
            return false;
        }
        const byCall = this.resolveReqMap(reqId);
        const entry = this.resolveEntry(byCall, callId);
        if (item.type === RESP_EVENT_AGENT.ON_TOOL_CALL) {
            if (item.context.toolId) {
                entry.toolName = item.context.toolId;
            }
            if (isNil(entry.args)) {
                entry.args = this.resolveArgs(item.context.args);
            }
            return true;
        }
        if (item.type === RESP_EVENT_AGENT.ON_CONFLICT) {
            if (item.context.toolId) {
                entry.toolName = item.context.toolId;
            }
            entry.hasConflict = true;
            entry.args = this.resolveArgs(item.context.args);
            //自动审核放行不会再有答复事件，这里直接定案，否则回顾里会一直停在待确认
            if (item.context.autoAllowed) {
                entry.answerCode = AGENT_ANSWER_CODE.ALLOW;
            }
            return true;
        }
        if (item.type === RESP_EVENT_AGENT.ON_CONFLICT_ANSWERED) {
            if (item.context.toolId) {
                entry.toolName = item.context.toolId;
            }
            if (isNotNil(item.context.code)) {
                entry.answerCode = item.context.code;
            }
            return true;
        }
        if (item.type === RESP_EVENT_AGENT.ON_TOOL_CALL_DONE) {
            if (item.context.result) {
                entry.result = item.context.result;
            }
            return true;
        }
        return false;
    }

    buildLines(reqId: string): RunSummaryLine[] {
        const byCall = this.entriesByReq.get(reqId);
        if (!byCall) {
            return [];
        }
        const lines: RunSummaryLine[] = [];
        for (const entry of byCall.values()) {
            if (!entry.toolName) {
                continue;
            }
            const line = buildSummaryLine(entry);
            if (isEditTool(line.toolId)) {
                lines.push(...expandEditLines(entry, line));
                continue;
            }
            lines.push(line);
        }
        return lines;
    }

    private resolveReqMap(reqId: string): Map<string, RunSummaryEntry> {
        const existing = this.entriesByReq.get(reqId);
        if (isNotNil(existing)) {
            return existing;
        }
        const created = new Map<string, RunSummaryEntry>();
        this.entriesByReq.set(reqId, created);
        return created;
    }

    private resolveEntry(byCall: Map<string, RunSummaryEntry>, callId: string): RunSummaryEntry {
        const existing = byCall.get(callId);
        if (isNotNil(existing)) {
            return existing;
        }
        const created: RunSummaryEntry = {
            callId,
            toolName: '',
            args: null,
            hasConflict: false,
            answerCode: null,
            result: null,
        };
        byCall.set(callId, created);
        return created;
    }

    private resolveArgs(args: unknown): SummaryArgsRaw | null {
        if (!args) {
            return null;
        }
        return args as SummaryArgsRaw;
    }
}

export function canShowRunSummary(index: number, total: number, runDone: boolean): boolean {
    if (index < total - 1) {
        return true;
    }
    return runDone;
}

export function resolveTurnReqId(turn: AiChatItem[]): string | null {
    for (const item of turn) {
        if (item.type !== RESP_EVENT_AGENT.ON_USER) {
            continue;
        }
        const reqId = item.context.reqId;
        if (!reqId) {
            return null;
        }
        return reqId;
    }
    return null;
}
