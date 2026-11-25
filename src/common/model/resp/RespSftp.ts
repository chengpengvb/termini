import type {FileEntry} from "../dto/FileEntry";
import type {FileStatProperties} from "../dto/FileStatProperties";
import {OpenStep} from "@/common/model/resp/event/RespEventOpenStep.ts";

export interface RespSftpBase {
    path: string;
    respId?: string | null;
    clientTransferId: string;
}

export interface RespSftpOpenStep {
    step: OpenStep;
    clientTransferId: string;
}

export interface RespSftpEntries {
    hasParent: boolean;
    entries: FileEntry[];
    clientTransferId: string;
    parentPath?: string | null;
}

export interface RespSftpError {
    message: string;
    clientTransferId: string;
}

export interface RespSftpEntry {
    entry: FileEntry;
    clientTransferId: string;
}

export interface RespSftpStatProperties {
    clientTransferId: string;
    properties: FileStatProperties;
}

export interface RespSftpTfProgress {
    file: string;
    speed: number;
    respId: string;
    finish: boolean;
    percent: number;
    filesDone: number;
    filesTotal: number;
    transferTotal: number;
    transferDone: number;
    clientTransferId: string;
}

export interface RespSftpTransferCount {
    status: number;
    total: number;
    clientTransferId: string;
}

export const SFTP_TRANSFER_COUNT_STATUS = {
    COUNTING: 0,
    DONE: 1,
} as const;

export interface RespSftpTransferConflict {
    path: string;
    requestId: string;
    clientTransferId: string;
}

export interface RespSftpDeleteProgress {
    file: string;
    finish: boolean;
    percent: number;
    respId: string;
    filesDone: number;
    filesTotal: number;
    clientTransferId: string;
}

export interface RespSftpDeleteCount {
    status: number;
    total: number;
    clientTransferId: string;
}

export const SFTP_DELETE_COUNT_STATUS = {
    COUNTING: 0,
    DONE: 1,
} as const;

export interface RespSftpFileOpened {
    path: string;
    editId: string | null;
    clientTransferId: string;
}

export interface RespSftpLocalId {
    clientTransferId: string;
}
