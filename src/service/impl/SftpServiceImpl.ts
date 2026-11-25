import {inject, injectable} from "inversify";
import {Dispatch, SetStateAction} from "react";
import {API_BASE_URL} from "@/runtime-config.ts";
import {SftpService} from "@/service/SftpService.ts";
import {SftpMeta} from "@/common/model/dto/SftpMeta.ts";
import {StepEvent} from "@/page/common/dto/terminal/StepEvent";
import {PathNavState} from "@/common/model/dto/PathNavState.ts";
import {SshServiceImpl} from "@/service/impl/SshServiceImpl.ts";
import {AuthServiceImpl} from "@/service/impl/AuthServiceImpl.ts";
import {BaseServiceImpl} from "@/service/impl/BaseServiceImpl.ts";
import {REQ_WS_EVENT_SFTP} from "@/common/model/req/ReqWsEvent.ts";
import {EntrySortState} from "@/common/model/dto/EntrySortState.ts";
import {ReqSocketContext} from "@/common/model/req/ReqWsContext.ts";
import {RespSocketContext} from "@/common/model/resp/RespWsContext";
import {SshIdentityMaps} from "@/common/model/dto/SshIdentityMaps.ts";
import {BaseApiServiceImpl} from "@/service/impl/BaseApiServiceImpl.ts";
import {RESP_EVENT_SFTP} from "@/common/model/resp/event/RespEventSftp.ts";
import {WebSocketServiceImpl} from "@/service/impl/WebSocketServiceImpl.ts";
import {BizError, ResultModel, SUCCESS_CODE} from "@/common/model/resp/ResultModel.ts";
import {RespSftpFileCopy, RespSftpFilePaste} from "@/common/model/resp/RespSftpTransfer.ts";
import {RespSftpBase, RespSftpLocalId, RespSftpOpenStep} from "@/common/model/resp/RespSftp.ts";
import {SftpChmodEvent, SftpCreateEvent, SftpDeleteCancelEvent, SftpDeleteEvent, SftpExportEvent, SftpFileOpenEvent, SftpHomeDirEvent, SftpListDirEvent, SftpRenameEvent, SftpTransferAnswerEvent, SftpTransferCancelEvent, SftpTransferEvent} from "@/common/model/req/event/WsSftp.ts";

@injectable()
export class SftpServiceImpl extends BaseServiceImpl implements SftpService {

    private readonly PATH_HISTORY_MAX_COUNT = 20;
    private localClientTransferId: string | undefined;
    private readonly PATH_HISTORY_STORAGE_PREFIX = 'argus:sftp:path-history:';
    private readonly closeListeners: Map<string, Set<() => void>> = new Map();
    private readonly connectRetryListeners: Map<string, Set<() => void>> = new Map();
    private readonly sessionId2SftpMeta: Map<string, SftpMeta> = new Map<string, SftpMeta>();
    private readonly sessionId2PathNav: Map<string, PathNavState> = new Map<string, PathNavState>();
    private readonly sessionId2EntrySort: Map<string, EntrySortState> = new Map<string, EntrySortState>();
    private readonly eventListeners: Map<string, Set<(arg: { type: number, data: any }) => void>> = new Map();
    private readonly sessionId2PathScroll: Map<string, Map<string, number>> = new Map<string, Map<string, number>>();

    constructor(
        @inject(SshServiceImpl) private sshService: SshServiceImpl,
        @inject(AuthServiceImpl) private authService: AuthServiceImpl,
        @inject(WebSocketServiceImpl) private webSocketService: WebSocketServiceImpl) {
        super();
    }


    noticeRetry(session_id: string): void {
        this.authService.emitEvent(this.connectRetryListeners, session_id, undefined)
    }

    addCloseListener(session_id: string, cb: () => void): () => void {
        return this.authService.addSessionListener(this.closeListeners, session_id, cb);
    }

    addConnectRetryListener(session_id: string, cb: () => void): () => void {
        return this.authService.addSessionListener(this.connectRetryListeners, session_id, cb);
    }

    addAuthListener(session_id: string, cb: (context: RespSocketContext) => void): () => void {
        return this.authService.addAuthListener(session_id, cb);
    }

    addConnectStepListener(session_id: string, cb: (stepEvent: StepEvent) => void): () => void {
        return this.authService.addConnectStepListener(session_id, cb);
    }

    addEventListener(session_id: string, callback: (arg: { type: number, data: any }) => void): () => void {
        return this.authService.addSessionListener(this.eventListeners, session_id, callback);
    }

    addWsListener(session_id: string, cb: (context: RespSocketContext) => void): () => void {
        return this.webSocketService.addWsListener((context) => {
            if (context.session_id !== session_id) {
                return;
            }
            cb(context);
        });
    }

    close(session_id: string) {
        this.sessionId2SftpMeta.delete(session_id);
        const message: ReqSocketContext = {
            session_id: session_id,
            type: REQ_WS_EVENT_SFTP.CLOSE,
        };
        this.webSocketService.send(message);
    }

    getCurrentPath(session_id: string): string | undefined {
        let sftpMeta = this.sessionId2SftpMeta.get(session_id)
        if (!sftpMeta) {
            return undefined
        }
        return sftpMeta.currentPath
    }

    pushPathNav(session_id: string, path: string): void {
        if (!path) {
            return;
        }
        let state = this.sessionId2PathNav.get(session_id);
        if (!state) {
            this.sessionId2PathNav.set(session_id, {entries: [path], index: 0, pendingHistoryNav: false});
            return;
        }
        if (state.pendingHistoryNav) {
            state.pendingHistoryNav = false;
            return;
        }
        const currentPath = state.entries[state.index];
        if (currentPath === path) {
            return;
        }
        state.entries = [...state.entries.slice(0, state.index + 1), path];
        state.index = state.entries.length - 1;
    }

    navigatePathBack(session_id: string): string | null {
        const state = this.sessionId2PathNav.get(session_id);
        if (!state || state.index <= 0) {
            return null;
        }
        state.index -= 1;
        state.pendingHistoryNav = true;
        return state.entries[state.index];
    }

    navigatePathForward(session_id: string): string | null {
        const state = this.sessionId2PathNav.get(session_id);
        if (!state || state.index >= state.entries.length - 1) {
            return null;
        }
        state.index += 1;
        state.pendingHistoryNav = true;
        return state.entries[state.index];
    }

    canPathNavigateBack(session_id: string): boolean {
        const state = this.sessionId2PathNav.get(session_id);
        if (!state) {
            return false;
        }
        return state.index > 0;
    }

    canPathNavigateForward(session_id: string): boolean {
        const state = this.sessionId2PathNav.get(session_id);
        if (!state) {
            return false;
        }
        return state.index < state.entries.length - 1;
    }

    clearSessionUiState(session_id: string): void {
        this.sessionId2PathNav.delete(session_id);
        this.sessionId2PathScroll.delete(session_id);
        this.sessionId2EntrySort.delete(session_id);
    }

    savePathScrollTop(session_id: string, path: string, scrollTop: number): void {
        if (!path) {
            return;
        }
        let scrollByPath = this.sessionId2PathScroll.get(session_id);
        if (!scrollByPath) {
            scrollByPath = new Map<string, number>();
            this.sessionId2PathScroll.set(session_id, scrollByPath);
        }
        scrollByPath.set(path, scrollTop);
    }

    getPathScrollTop(session_id: string, path: string): number | null {
        if (!path) {
            return null;
        }
        const scrollByPath = this.sessionId2PathScroll.get(session_id);
        if (!scrollByPath) {
            return null;
        }
        const scrollTop = scrollByPath.get(path);
        if (scrollTop === undefined) {
            return null;
        }
        return scrollTop;
    }

    saveEntrySort(session_id: string, sort: EntrySortState): void {
        this.sessionId2EntrySort.set(session_id, {column: sort.column, order: sort.order});
    }

    getEntrySort(session_id: string): EntrySortState | null {
        const sort = this.sessionId2EntrySort.get(session_id);
        if (!sort) {
            return null;
        }
        return {column: sort.column, order: sort.order};
    }

    getPathNavCurrentPath(session_id: string): string | null {
        const state = this.sessionId2PathNav.get(session_id);
        if (!state) {
            return null;
        }
        const path = state.entries[state.index];
        if (!path) {
            return null;
        }
        return path;
    }

    isPathHistoryNavPending(session_id: string): boolean {
        const state = this.sessionId2PathNav.get(session_id);
        if (!state) {
            return false;
        }
        return state.pendingHistoryNav;
    }

    getPathHistory(deviceId: string): string[] {
        return this.loadPathHistory(deviceId);
    }

    addPathHistory(deviceId: string, path: string): string[] {
        if (!path) {
            return this.loadPathHistory(deviceId);
        }
        const next = [path, ...this.loadPathHistory(deviceId).filter(item => item !== path)].slice(0, this.PATH_HISTORY_MAX_COUNT);
        this.persistPathHistory(deviceId, next);
        return next;
    }

    removePathHistory(deviceId: string, path: string): string[] {
        const next = this.loadPathHistory(deviceId).filter(item => item !== path);
        this.persistPathHistory(deviceId, next);
        return next;
    }

    getClientTransferId(session_id: string): string | undefined {
        let sftpMeta = this.sessionId2SftpMeta.get(session_id)
        if (!sftpMeta) {
            return undefined
        }
        if (sftpMeta.respSftpOpen) {
            return sftpMeta.respSftpOpen.clientTransferId
        }
        return sftpMeta.clientTransferId
    }

    getDeviceId(session_id: string): string | undefined {
        const sftpMeta = this.sessionId2SftpMeta.get(session_id);
        if (!sftpMeta) {
            return undefined;
        }
        if (!sftpMeta.sftpOpenEvent) {
            return undefined;
        }
        return sftpMeta.sftpOpenEvent.data.deviceId;
    }

    async getIdentityMap(deviceId: string): Promise<SshIdentityMaps | null> {
        if (!deviceId) {
            return null;
        }
        try {
            const res = await fetch(`${API_BASE_URL}/sftp/identity/map/${deviceId}`, {headers: BaseApiServiceImpl.buildAuthHeaders()});
            if (!res.ok) {
                return null;
            }
            const body = await res.json() as ResultModel<SshIdentityMaps>;
            if (body.code !== SUCCESS_CODE || !body.data) {
                return null;
            }
            return body.data;
        } catch {
            return null;
        }
    }

    async getLocalClientTransferId(): Promise<string | undefined> {
        if (this.localClientTransferId) {
            return this.localClientTransferId;
        }
        const res = await fetch(`${API_BASE_URL}/sftp/local_client_transfer_id`, {headers: BaseApiServiceImpl.buildAuthHeaders()});
        if (!res.ok) {
            return undefined;
        }
        const body = await res.json() as { code: number; data: RespSftpLocalId | null; message: string };
        if (body.code !== SUCCESS_CODE || !body.data) {
            return undefined;
        }
        this.localClientTransferId = body.data.clientTransferId;
        return this.localClientTransferId;
    }

    async setFileCopy(payload: RespSftpFileCopy): Promise<void> {
        const res = await fetch(`${API_BASE_URL}/sftp/file_copy`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json', ...BaseApiServiceImpl.buildAuthHeaders()},
            body: JSON.stringify(payload),
        });
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const body = await res.json() as ResultModel<null>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
    }

    async setFilePaste(payload: RespSftpFilePaste): Promise<void> {
        const res = await fetch(`${API_BASE_URL}/sftp/file_paste`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json', ...BaseApiServiceImpl.buildAuthHeaders()},
            body: JSON.stringify(payload),
        });
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const body = await res.json() as ResultModel<null>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
    }

    async handleConnect(session_id: string, payload: any, handle: any, setOpened: Dispatch<SetStateAction<boolean>>): Promise<() => void> {
        const handleResp = this.addWsListener(session_id, (resp: RespSocketContext) => {
            this.handleRespContext(handle, resp, setOpened)
        });
        this.handleOpen(session_id, payload).then()
        return () => {
            handleResp();
            this.close(session_id);
        }
    }

    async handleOpen(session_id: string, payload: any): Promise<void> {
        this.authService.clearConnectSteps(session_id);
        this.sessionId2SftpMeta.set(session_id, {sftpOpenEvent: payload});
        const message: ReqSocketContext = {
            payload: payload,
            session_id: session_id,
            type: REQ_WS_EVENT_SFTP.OPEN,
        };
        this.webSocketService.send(message);
    }

    async handleHomeDir(session_id: string, payload: any): Promise<void> {
        const message: ReqSocketContext = {
            payload: payload,
            session_id: session_id,
            type: REQ_WS_EVENT_SFTP.HOME_DIR,
        };
        this.webSocketService.send(message);
    }

    handleTransfer(session_id: string, payload: SftpTransferEvent): void {
        const message: ReqSocketContext = {
            payload: payload,
            session_id: session_id,
            type: REQ_WS_EVENT_SFTP.TRANSFER,
        };
        this.webSocketService.send(message);
    }

    handleDelete(session_id: string, payload: SftpDeleteEvent): void {
        const message: ReqSocketContext = {
            payload: payload,
            session_id: session_id,
            type: REQ_WS_EVENT_SFTP.DELETE,
        };
        this.webSocketService.send(message);
    }

    handleRename(session_id: string, payload: SftpRenameEvent): void {
        const message: ReqSocketContext = {
            payload: payload,
            session_id: session_id,
            type: REQ_WS_EVENT_SFTP.RENAME,
        };
        this.webSocketService.send(message);
    }

    handleCreate(session_id: string, payload: SftpCreateEvent): void {
        const message: ReqSocketContext = {
            payload: payload,
            session_id: session_id,
            type: REQ_WS_EVENT_SFTP.CREATE,
        };
        this.webSocketService.send(message);
    }

    handleChmod(session_id: string, payload: SftpChmodEvent): void {
        const message: ReqSocketContext = {
            payload: payload,
            session_id: session_id,
            type: REQ_WS_EVENT_SFTP.CHMOD,
        };
        this.webSocketService.send(message);
    }

    handleFileOpen(session_id: string, payload: SftpFileOpenEvent): void {
        const message: ReqSocketContext = {
            payload: payload,
            session_id: session_id,
            type: REQ_WS_EVENT_SFTP.FILE_OPEN,
        };
        this.webSocketService.send(message);
    }

    handleExportToTemp(session_id: string, payload: SftpExportEvent): void {
        const message: ReqSocketContext = {
            payload: payload,
            session_id: session_id,
            type: REQ_WS_EVENT_SFTP.EXPORT_TO_TEMP,
        };
        this.webSocketService.send(message);
    }

    handleTransferAnswer(session_id: string, payload: SftpTransferAnswerEvent): void {
        const message: ReqSocketContext = {
            payload: payload,
            session_id: session_id,
            type: REQ_WS_EVENT_SFTP.TRANSFER_ANSWER,
        };
        this.webSocketService.send(message);
    }

    handleTransferCancel(session_id: string, payload: SftpTransferCancelEvent): void {
        const message: ReqSocketContext = {
            payload: payload,
            session_id: session_id,
            type: REQ_WS_EVENT_SFTP.TRANSFER_CANCEL,
        };
        this.webSocketService.send(message);
    }

    handleDeleteCancel(session_id: string, payload: SftpDeleteCancelEvent): void {
        const message: ReqSocketContext = {
            payload: payload,
            session_id: session_id,
            type: REQ_WS_EVENT_SFTP.DELETE_CANCEL,
        };
        this.webSocketService.send(message);
    }

    handleListDir(session_id: string, path: string): void {
        let payload: SftpListDirEvent = {
            data: {path: path, clientTransferId: this.getClientTransferId(session_id)}
        }
        const message: ReqSocketContext = {
            payload: payload,
            session_id: session_id,
            type: REQ_WS_EVENT_SFTP.LIST_DIR,
        };
        this.webSocketService.send(message);
    }

    notifyLocalError(session_id: string, message: string): void {
        if (!message) {
            return;
        }
        this.authService.emitEvent(this.eventListeners, session_id, {type: RESP_EVENT_SFTP.ON_ERROR, data: {message, clientTransferId: ''}});
    }

    handleRespContext(_handle: any, context: RespSocketContext, setOpened: Dispatch<SetStateAction<boolean>>) {
        const type = context.type
        if (type == RESP_EVENT_SFTP.ON_OPEN) {
            this.handleOnOpenEvent(context, setOpened)
        } else if (type == RESP_EVENT_SFTP.ON_CLOSE) {
            this.handleOnCloseEvent(context, setOpened)
        } else if (type == RESP_EVENT_SFTP.ON_OPEN_STEP) {
            this.handleOnOpenStepEvent(context)
        } else {
            this.handleOnPathEvent(type, context)
            this.authService.emitEvent(this.eventListeners, context.session_id, {type, data: context.payload})
        }
    }

    handleOnOpenEvent(context: RespSocketContext, setOpened: Dispatch<SetStateAction<boolean>>) {
        setOpened(true)
        const sessionId = context.session_id
        let sftpMeta = this.sessionId2SftpMeta.get(sessionId)
        if (!sftpMeta) {
            //TODO 报错
            return
        }
        sftpMeta.respSftpOpen = context.payload as RespSftpBase
        const payload: SftpHomeDirEvent = {data: {clientTransferId: sftpMeta.respSftpOpen.clientTransferId}}
        this.handleHomeDir(sessionId, payload).then()
    }

    handleOnCloseEvent(context: RespSocketContext, setOpened: Dispatch<SetStateAction<boolean>>) {
        setOpened(false)
        this.authService.emitEvent(this.closeListeners, context.session_id, undefined)
    }

    private buildInnerContext(context: RespSocketContext): RespSocketContext {
        let respSftpOpenStep = context.payload as RespSftpOpenStep;
        return {
            ...context,
            payload: respSftpOpenStep.step,
        };
    }

    private handleOnOpenStepEvent(context: RespSocketContext): void {
        const respSftpOpenStep = context.payload as RespSftpOpenStep;
        const sessionId = context.session_id;
        let sftpMeta = this.sessionId2SftpMeta.get(sessionId);
        if (sftpMeta) {
            sftpMeta.clientTransferId = respSftpOpenStep.clientTransferId;
        }
        console.log(context)
        let innerContext = this.buildInnerContext(context)
        this.authService.onOpenStep(innerContext, this.sshService.isAuthType.bind(this))
        this.authService.notifyConnectStep(context.session_id, this.sshService.parser2progress(innerContext))
    }

    private handleOnPathEvent(type: number, context: RespSocketContext): void {
        if (type != RESP_EVENT_SFTP.ON_PATH) {
            return;
        }
        const sessionId = context.session_id
        let sftpMeta = this.sessionId2SftpMeta.get(sessionId)
        const path = (context.payload as RespSftpBase).path;
        if (!sftpMeta) {
            sftpMeta = {currentPath: path}
            this.sessionId2SftpMeta.set(sessionId, sftpMeta)
            return
        }
        sftpMeta.currentPath = path
    }

    private resolvePathHistoryStorageKey(deviceId: string): string {
        return `${this.PATH_HISTORY_STORAGE_PREFIX}${deviceId}`;
    }

    private loadPathHistory(deviceId: string): string[] {
        const raw = window.localStorage.getItem(this.resolvePathHistoryStorageKey(deviceId));
        if (!raw) {
            return [];
        }
        try {
            const parsed = JSON.parse(raw);
            if (!Array.isArray(parsed)) {
                return [];
            }
            return (parsed as string[]).filter(item => item.length > 0);
        } catch {
            return [];
        }
    }

    private persistPathHistory(deviceId: string, paths: string[]): void {
        window.localStorage.setItem(this.resolvePathHistoryStorageKey(deviceId), JSON.stringify(paths));
    }

}