export interface SftpOpen {
    deviceId: string;
    deviceType: number;
    protocol: number;
}

export interface SftpClientRef {
    clientTransferId: string | undefined;
}

export interface SftpPathData {
    path: string;
    clientTransferId: string | undefined;
}

export interface SftpStatData {
    path: string;
    kind: string;
    clientTransferId: string | undefined;
}

export interface SftpRenameData {
    path: string;
    fromName: string;
    toName: string;
    clientTransferId: string | undefined;
}

export interface SftpChmodData {
    mode: number;
    path: string;
    names: string[];
    clientTransferId: string | undefined;
}

export interface SftpCreateData {
    name: string;
    path: string;
    kind: string;
    linkTarget?: string | null;
    clientTransferId: string | undefined;
}

export interface SftpDeleteData {
    path: string;
    names: string[];
    clientTransferId: string | undefined;
}

export interface SftpTransferData {
    names: string[];
    sourcePath: string;
    targetPath: string;
    sideSessionId: string;
    sourceClientTransferId: string | undefined;
    targetClientTransferId: string | undefined;
}

export interface SftpTransferAnswerData {
    code: number;
    requestId: string;
}

export interface SftpTransferCancelData {
    respId: string;
}

export interface SftpDeleteCancelData {
    respId: string;
}

export const SFTP_TRANSFER_ANSWER = {
    OVERWRITE: 0,
    OVERWRITE_ALL: 1,
    SKIP: 2,
    SKIP_ALL: 3,
} as const;

export interface SftpFileOpenData {
    path: string;
    openWith: boolean;
    clientTransferId: string | undefined;
}

export interface SftpExportData {
    names: string[];
    sourcePath: string;
    sourceClientTransferId: string;
}

export interface SftpOpenEvent {
    data: SftpOpen;
}

export interface SftpHomeDirEvent {//获取用Home目录
    data: SftpClientRef;
}

export interface SftpListDirEvent {
    data: SftpPathData;
}

export interface SftpStatEvent {
    data: SftpStatData;
}

export interface SftpRenameEvent {
    data: SftpRenameData;
}

export interface SftpChmodEvent {
    data: SftpChmodData;
}

export interface SftpCreateEvent {
    data: SftpCreateData;
}

export interface SftpDeleteEvent {
    data: SftpDeleteData;
}

export interface SftpTransferEvent {
    data: SftpTransferData;
}

export interface SftpTransferAnswerEvent {
    data: SftpTransferAnswerData;
}

export interface SftpTransferCancelEvent {
    data: SftpTransferCancelData;
}

export interface SftpDeleteCancelEvent {
    data: SftpDeleteCancelData;
}

export interface SftpFileOpenEvent {
    data: SftpFileOpenData;
}

export interface SftpExportEvent {
    data: SftpExportData;
}
