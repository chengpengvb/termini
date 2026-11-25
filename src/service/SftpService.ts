import {ConnectService} from "@/service/ConnectService.ts";
import type {EntrySortState} from "@/common/model/dto/EntrySortState.ts";
import type {SshIdentityMaps} from "@/common/model/dto/SshIdentityMaps.ts";
import type {RespSftpFileCopy, RespSftpFilePaste} from "@/common/model/resp/RespSftpTransfer.ts";
import {SftpChmodEvent, SftpCreateEvent, SftpDeleteCancelEvent, SftpDeleteEvent, SftpExportEvent, SftpFileOpenEvent, SftpRenameEvent, SftpTransferAnswerEvent, SftpTransferCancelEvent, SftpTransferEvent} from "@/common/model/req/event/WsSftp.ts";

export interface SftpService extends ConnectService {
    close(session_id: string): void;

    clearSessionUiState(session_id: string): void;

    getPathHistory(deviceId: string): string[];

    canPathNavigateBack(session_id: string): boolean;

    canPathNavigateForward(session_id: string): boolean;

    pushPathNav(session_id: string, path: string): void;

    navigatePathBack(session_id: string): string | null;

    getDeviceId(session_id: string): string | undefined;

    isPathHistoryNavPending(session_id: string): boolean;

    navigatePathForward(session_id: string): string | null;

    handleListDir(session_id: string, path: string): void;

    setFileCopy(payload: RespSftpFileCopy): Promise<void>;

    setFilePaste(payload: RespSftpFilePaste): Promise<void>;

    getLocalClientTransferId(): Promise<string | undefined>;

    getCurrentPath(session_id: string): string | undefined;

    getEntrySort(session_id: string): EntrySortState | null;

    getPathNavCurrentPath(session_id: string): string | null;

    addPathHistory(deviceId: string, path: string): string[];

    removePathHistory(deviceId: string, path: string): string[];

    getClientTransferId(session_id: string): string | undefined;

    notifyLocalError(session_id: string, message: string): void;

    saveEntrySort(session_id: string, sort: EntrySortState): void;

    handleChmod(session_id: string, payload: SftpChmodEvent): void;

    getIdentityMap(deviceId: string): Promise<SshIdentityMaps | null>;

    getPathScrollTop(session_id: string, path: string): number | null;

    handleDelete(session_id: string, payload: SftpDeleteEvent): void;

    handleRename(session_id: string, payload: SftpRenameEvent): void;

    handleCreate(session_id: string, payload: SftpCreateEvent): void;

    handleTransfer(session_id: string, payload: SftpTransferEvent): void;

    handleFileOpen(session_id: string, payload: SftpFileOpenEvent): void;

    handleExportToTemp(session_id: string, payload: SftpExportEvent): void;

    savePathScrollTop(session_id: string, path: string, scrollTop: number): void;

    handleDeleteCancel(session_id: string, payload: SftpDeleteCancelEvent): void;

    handleTransferAnswer(session_id: string, payload: SftpTransferAnswerEvent): void;

    handleTransferCancel(session_id: string, payload: SftpTransferCancelEvent): void;

    addEventListener(session_id: string, callback: (arg: { type: number, data: any }) => void): () => void;
}
