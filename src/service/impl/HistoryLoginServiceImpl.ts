import {injectable} from 'inversify';
import {PROTOCOL} from '@/common/const/Const.ts';
import {BaseListenerService} from './BaseListenerService';
import {isEmpty, isNil} from '@/common/util/StringUtil.ts';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import type {HistoryLoginService} from '../HistoryLoginService';
import type {HistoryLogin} from '@/common/model/dto/HistoryLogin.ts';

@injectable()
export class HistoryLoginServiceImpl extends BaseListenerService implements HistoryLoginService {

    private items: HistoryLogin[] = [];
    private readonly MAX_SIZE = 100;
    private readonly KEY_LIST_CHANGE = 'history-login-list-change';
    private readonly STORAGE_KEY = 'argus:history-login:list';
    private readonly VALID_PROTOCOLS = new Set<number>([PROTOCOL.SSH, PROTOCOL.TELNET, PROTOCOL.SFTP, PROTOCOL.RDP, PROTOCOL.FTP]);

    constructor() {
        super();
        this.items = this.load();
        window.localStorage.setItem(this.STORAGE_KEY, JSON.stringify(this.items));
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.NONE;
    }

    list(): HistoryLogin[] {
        return this.items.slice();
    }

    clear(): void {
        if (this.items.length === 0) {
            return;
        }
        this.items = [];
        this.persistAndNotify();
    }

    add(deviceId: string, protocol: number, time?: number): void {
        if (isEmpty(deviceId) || !this.VALID_PROTOCOLS.has(protocol)) {
            return;
        }
        const nextTime = time ?? Date.now();
        const rest = this.items.filter((item) => item.device_id !== deviceId || item.protocol !== protocol);
        this.items = [{device_id: deviceId, protocol, time: nextTime}, ...rest].slice(0, this.MAX_SIZE);
        this.persistAndNotify();
    }

    removeByDeviceId(deviceId: string): void {
        if (isEmpty(deviceId)) {
            return;
        }
        const next = this.items.filter((item) => item.device_id !== deviceId);
        if (next.length === this.items.length) {
            return;
        }
        this.items = next;
        this.persistAndNotify();
    }

    getLastProtocols(deviceId: string): number[] {
        if (isEmpty(deviceId)) {
            return [];
        }
        const matched = this.items.filter((item) => item.device_id === deviceId);
        if (matched.length === 0) {
            return [];
        }
        const latestTime = matched.reduce((max, item) => (item.time > max ? item.time : max), matched[0].time);
        const protocols: number[] = [];
        for (const item of matched) {
            if (item.time !== latestTime) {
                continue;
            }
            if (protocols.includes(item.protocol)) {
                continue;
            }
            protocols.push(item.protocol);
        }
        return protocols;
    }

    addListChangeListener(listener: (list: HistoryLogin[]) => void): () => void {
        return this.addListener<HistoryLogin[]>(this.KEY_LIST_CHANGE, listener);
    }

    private load(): HistoryLogin[] {
        const raw = window.localStorage.getItem(this.STORAGE_KEY);
        if (!raw) {
            return [];
        }
        try {
            const parsed = JSON.parse(raw) as HistoryLogin[];
            if (!Array.isArray(parsed)) {
                return [];
            }
            return this.dedupe(parsed.filter((item) => this.isHistoryLogin(item)));
        } catch {
            return [];
        }
    }

    private dedupe(items: HistoryLogin[]): HistoryLogin[] {
        const sorted = items.slice().sort((a, b) => b.time - a.time);
        const seen = new Set<string>();
        const next: HistoryLogin[] = [];
        for (const item of sorted) {
            const key = `${item.device_id}:${item.protocol}`;
            if (seen.has(key)) {
                continue;
            }
            seen.add(key);
            next.push(item);
            if (next.length >= this.MAX_SIZE) {
                break;
            }
        }
        return next;
    }

    private isHistoryLogin(item: HistoryLogin): boolean {
        if (isNil(item)) {
            return false;
        }
        if (isEmpty(item.device_id)) {
            return false;
        }
        if (!this.VALID_PROTOCOLS.has(item.protocol)) {
            return false;
        }
        return Number.isFinite(item.time);
    }

    private persistAndNotify(): void {
        window.localStorage.setItem(this.STORAGE_KEY, JSON.stringify(this.items));
        this.notifyListeners<HistoryLogin[]>(this.KEY_LIST_CHANGE, this.list());
    }
}
