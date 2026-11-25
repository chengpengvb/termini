import {inject, injectable} from 'inversify';
import type {ApiService} from '../ApiService';
import {API_BASE_URL} from '@/runtime-config.ts';
import {isNil} from "@/common/util/StringUtil.ts";
import {BaseListenerService} from './BaseListenerService';
import type {ResultModel} from '@/common/model/resp/ResultModel.ts';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';
import type {RespSocketContext} from '@/common/model/resp/RespWsContext.ts';
import {BizError, RESULT_CODE, SUCCESS_CODE} from '@/common/model/resp/ResultModel.ts';
import {isEntityNotifyEvent} from '@/common/model/resp/event/RespEventEntityNotify.ts';

@injectable()
export abstract class BaseApiServiceImpl<T = unknown> extends BaseListenerService implements ApiService<T> {
    private static apiToken: string | null = null;
    private static readonly TOKEN_RESTORE_DELAY_MS = 150;
    private static readonly TOKEN_RESTORE_MAX_ATTEMPTS = 60;
    private static readonly TOKEN_RESTORE_WARMING_UP_CODE = 503;
    private static unauthorizedHandler: (() => void) | null = null;

    protected readonly JSON_HEADERS: Record<string, string> = {
        'Content-Type': 'application/json',
    };

    constructor(
        @inject(WebSocketServiceImpl) webSocketService: WebSocketServiceImpl,
    ) {
        super();
        webSocketService.addWsListener((context: RespSocketContext) => {
            if (!isEntityNotifyEvent(context.type)) {
                return;
            }
            this.handleEntityNotifyPayload(context.payload);
        });
    }

    static setApiToken(token: string | null): void {
        BaseApiServiceImpl.apiToken = token;
    }

    static setUnauthorizedHandler(handler: (() => void) | null): void {
        BaseApiServiceImpl.unauthorizedHandler = handler;
    }

    protected static handleUnauthorized(): void {
        BaseApiServiceImpl.apiToken = null;
        if (BaseApiServiceImpl.unauthorizedHandler !== null) {
            BaseApiServiceImpl.unauthorizedHandler();
        }
    }

    static buildAuthHeaders(): Record<string, string> {
        if (BaseApiServiceImpl.apiToken === null) {
            return {};
        }
        return {Authorization: `Bearer ${BaseApiServiceImpl.apiToken}`};
    }

    static async restoreApiToken(): Promise<void> {
        for (let attempt = 0; attempt < BaseApiServiceImpl.TOKEN_RESTORE_MAX_ATTEMPTS; attempt++) {
            const body = await BaseApiServiceImpl.fetchApiToken();
            if (body === null || body.code === BaseApiServiceImpl.TOKEN_RESTORE_WARMING_UP_CODE) {
                await BaseApiServiceImpl.delay(BaseApiServiceImpl.TOKEN_RESTORE_DELAY_MS);
                continue;
            }
            BaseApiServiceImpl.setApiToken(body.data ?? null);
            return;
        }
        BaseApiServiceImpl.setApiToken(null);
    }

    private static delay(ms: number): Promise<void> {
        return new Promise((resolve) => setTimeout(resolve, ms));
    }

    private static async fetchApiToken(): Promise<ResultModel<string | null> | null> {
        try {
            const res = await fetch(`${API_BASE_URL}/account/token`);
            return (await res.json()) as ResultModel<string | null>;
        } catch {
            return null;
        }
    }

    private static async send(url: string, init?: RequestInit): Promise<Response> {
        const res = await fetch(url, {
            ...init,
            headers: {...BaseApiServiceImpl.buildAuthHeaders(), ...(init?.headers as Record<string, string>)},
        });
        if (res.status === RESULT_CODE.UNAUTHORIZED) {
            BaseApiServiceImpl.handleUnauthorized();
            throw new BizError(RESULT_CODE.UNAUTHORIZED);
        }
        if (!res.ok) {
            const text = await res.text();
            throw new Error(`HTTP ${res.status} ${url}: ${text}`);
        }
        return res;
    }

    private static readResult<R>(body: ResultModel<R>): R | null {
        if (body.code === RESULT_CODE.UNAUTHORIZED) {
            BaseApiServiceImpl.handleUnauthorized();
            throw new BizError(RESULT_CODE.UNAUTHORIZED);
        }
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code, body.message);
        }
        return body.data ?? null;
    }

    private static applyNdjsonLine<R>(line: string, onPartial: (data: R) => void, last: R | undefined): R | undefined {
        if (line.length === 0) {
            return last;
        }
        const data = BaseApiServiceImpl.readResult(JSON.parse(line) as ResultModel<R>);
        if (data === null) {
            throw new Error('ndjson line has no data');
        }
        onPartial(data);
        return data;
    }

    private static async readNdjson<R>(body: ReadableStream<Uint8Array>, onPartial: (data: R) => void): Promise<R> {
        const reader = body.getReader();
        const decoder = new TextDecoder();
        let buffer = '';
        let last: R | undefined;
        while (true) {
            const chunk = await reader.read();
            buffer += decoder.decode(chunk.value, {stream: !chunk.done});
            const lines = buffer.split('\n');
            const rest = lines.pop();
            if (rest === undefined) {
                throw new Error('ndjson split empty');
            }
            buffer = rest;
            for (const line of lines) {
                last = BaseApiServiceImpl.applyNdjsonLine(line, onPartial, last);
            }
            if (chunk.done) {
                break;
            }
        }
        if (buffer.length > 0) {
            last = BaseApiServiceImpl.applyNdjsonLine(buffer, onPartial, last);
        }
        if (last === undefined) {
            throw new Error('ndjson stream empty');
        }
        return last;
    }

    protected abstract getPrefix(): string | null;

    protected buildUrl(suffix: string): string {
        const prefix = this.getPrefix();
        if (isNil(prefix)) {
            return `${API_BASE_URL}${suffix}`;
        }
        return `${API_BASE_URL}${prefix}${suffix}`;
    }

    protected async request<R>(url: string, init?: RequestInit): Promise<R | null> {
        const res = await BaseApiServiceImpl.send(url, init);
        return BaseApiServiceImpl.readResult((await res.json()) as ResultModel<R>);
    }

    protected async requestNdjson<R>(url: string, init: RequestInit, onPartial: (data: R) => void): Promise<R> {
        const res = await BaseApiServiceImpl.send(url, init);
        const stream = res.body;
        if (!stream) {
            throw new Error(`${url} response has no body`);
        }
        return BaseApiServiceImpl.readNdjson(stream, onPartial);
    }

    async selectAll(): Promise<T[]> {
        const data = await this.request<T[]>(this.buildUrl('/select_all'));
        if (!data) {
            return [];
        }
        return data;
    }

    async selectById(id: string): Promise<T | null> {
        return this.request<T>(this.buildUrl(`/select/${id}`));
    }

    async insert(item: T): Promise<T> {
        const data = await this.request<T>(this.buildUrl('/insert'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify(item),
        });
        if (!data) {
            throw new Error('insert returned no data');
        }
        return data;
    }

    async update(item: T): Promise<T> {
        const data = await this.request<T>(this.buildUrl('/update'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify(item),
        });
        if (!data) {
            throw new Error('update returned no data');
        }
        return data;
    }

    async deleteById(id: string): Promise<void> {
        await this.request<unknown>(this.buildUrl(`/delete/${id}`), {method: 'DELETE'});
    }
}
