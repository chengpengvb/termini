import type {AiMcpServer} from '@/common/model/AiMcpServer.ts';
import {isEmpty, isNotEmpty} from '@/common/util/StringUtil.ts';
import {translate, translateFormat} from '@/common/i18n/i18n.ts';
import {MCP_TRANSPORT} from '@/common/const/McpTransportConst.ts';

interface McpJsonParseOk {
    ok: true;
    server: AiMcpServer;
}

interface McpJsonParseErr {
    ok: false;
    message: string;
}

export type McpJsonParseResult = McpJsonParseOk | McpJsonParseErr;

function isPlainObject(value: unknown): value is Record<string, unknown> {
    if (!value) {
        return false;
    }
    if (Array.isArray(value)) {
        return false;
    }
    return Object.getPrototypeOf(value) === Object.prototype;
}

function readString(value: unknown): string | null {
    if (!value && value !== '') {
        return null;
    }
    if (Object.prototype.toString.call(value) !== '[object String]') {
        return null;
    }
    const text = String(value).trim();
    if (!text) {
        return null;
    }
    return text;
}

function readStringArray(value: unknown): string[] | null {
    if (!value) {
        return null;
    }
    if (!Array.isArray(value)) {
        return null;
    }
    const out: string[] = [];
    for (const item of value) {
        const text = readString(item);
        if (!text && item !== '') {
            return null;
        }
        out.push(String(item));
    }
    return out;
}

function readStringMap(value: unknown): Record<string, string> | null {
    if (!value) {
        return null;
    }
    if (!isPlainObject(value)) {
        return null;
    }
    const out: Record<string, string> = {};
    for (const key of Object.keys(value)) {
        const text = readString(value[key]);
        if (!text) {
            return null;
        }
        out[key] = text;
    }
    return out;
}

function resolveJsonKey(name: string | null): string {
    if (isNotEmpty(name)) {
        return name.trim();
    }
    return 'server-name';
}

function buildEntry(server: AiMcpServer): Record<string, unknown> {
    const entry: Record<string, unknown> = {};
    if (server.transport === MCP_TRANSPORT.HTTP) {
        entry.type = 'http';
        if (isNotEmpty(server.url)) {
            entry.url = server.url.trim();
        }
        if (isNotEmpty(server.headers)) {
            try {
                const headers = JSON.parse(server.headers) as unknown;
                const map = readStringMap(headers);
                if (map) {
                    entry.headers = map;
                }
            } catch {
                // keep form headers as-is via UI; JSON side only emits valid object
            }
        }
    } else {
        if (isNotEmpty(server.command)) {
            entry.command = server.command.trim();
        }
        if (isNotEmpty(server.args)) {
            try {
                const args = JSON.parse(server.args) as unknown;
                const list = readStringArray(args);
                if (list) {
                    entry.args = list;
                }
            } catch {
                // ignore invalid args when serializing
            }
        }
        if (isNotEmpty(server.env)) {
            try {
                const env = JSON.parse(server.env) as unknown;
                const map = readStringMap(env);
                if (map) {
                    entry.env = map;
                }
            } catch {
                // ignore invalid env when serializing
            }
        }
    }
    if (!server.enabled) {
        entry.disabled = true;
    }
    return entry;
}

export function serializeMcpServerJson(server: AiMcpServer): string {
    const key = resolveJsonKey(server.name);
    return JSON.stringify({[key]: buildEntry(server)}, null, 2);
}

function resolveEntryBucket(root: unknown): McpJsonParseErr | {key: string; entry: Record<string, unknown>} {
    if (!isPlainObject(root)) {
        return {ok: false, message: translate('common.mcpJson.mustBeObject')};
    }
    if (isPlainObject(root.mcpServers)) {
        return pickNamedEntry(root.mcpServers);
    }
    if (readString(root.command) || readString(root.url)) {
        return {key: '', entry: root};
    }
    return pickNamedEntry(root);
}

function pickNamedEntry(bucket: Record<string, unknown>): McpJsonParseErr | {key: string; entry: Record<string, unknown>} {
    const keys = Object.keys(bucket);
    if (keys.length === 0) {
        return {ok: false, message: translate('common.mcpJson.noServerEntry')};
    }
    if (keys.length > 1) {
        return {ok: false, message: translate('common.mcpJson.singleServerOnly')};
    }
    const key = keys[0];
    const entry = bucket[key];
    if (!isPlainObject(entry)) {
        return {ok: false, message: translateFormat('common.mcpJson.entryMustBeObject', {key})};
    }
    return {key, entry};
}

function applyEntry(base: AiMcpServer, key: string, entry: Record<string, unknown>): McpJsonParseResult {
    const typeText = readString(entry.type);
    const url = readString(entry.url);
    const command = readString(entry.command);
    const isHttp = typeText === 'http' || (Boolean(url) && !command);
    if (isHttp && !url) {
        return {ok: false, message: translate('common.mcpJson.httpNeedsUrl')};
    }
    if (!isHttp && !command) {
        return {ok: false, message: translate('common.mcpJson.stdioNeedsCommand')};
    }

    let args: string | null = null;
    if (entry.args) {
        const list = readStringArray(entry.args);
        if (!list) {
            return {ok: false, message: translate('common.mcpJson.argsMustBeStringArray')};
        }
        args = JSON.stringify(list);
    }

    let env: string | null = null;
    if (entry.env) {
        const map = readStringMap(entry.env);
        if (!map) {
            return {ok: false, message: translate('common.mcpJson.envMustBeStringDict')};
        }
        env = JSON.stringify(map);
    }

    let headers: string | null = null;
    if (entry.headers) {
        const map = readStringMap(entry.headers);
        if (!map) {
            return {ok: false, message: translate('common.mcpJson.headersMustBeStringDict')};
        }
        headers = JSON.stringify(map);
    }

    const disabled = entry.disabled === true;
    const name = isNotEmpty(key) ? key : base.name;

    return {
        ok: true,
        server: {
            ...base,
            name: isEmpty(name) ? null : name,
            enabled: !disabled,
            transport: isHttp ? MCP_TRANSPORT.HTTP : MCP_TRANSPORT.STDIO,
            command: isHttp ? null : command,
            args: isHttp ? null : args,
            env: isHttp ? null : env,
            url: isHttp ? url : null,
            headers: isHttp ? headers : null,
        },
    };
}

export function parseMcpServerJson(text: string, base: AiMcpServer): McpJsonParseResult {
    const trimmed = text.trim();
    if (!trimmed) {
        return {ok: false, message: translate('common.mcpJson.cannotBeEmpty')};
    }
    let root: unknown;
    try {
        root = JSON.parse(trimmed);
    } catch {
        return {ok: false, message: translate('common.mcpJson.parseFailed')};
    }
    const bucket = resolveEntryBucket(root);
    const err = bucket as McpJsonParseErr;
    if (err.ok === false) {
        return err;
    }
    const resolved = bucket as {key: string; entry: Record<string, unknown>};
    return applyEntry(base, resolved.key, resolved.entry);
}
