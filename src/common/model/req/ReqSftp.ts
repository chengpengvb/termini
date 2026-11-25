export interface ReqSftpStartDrag {
    paths: string[];
}

export interface ReqSftpStartRemoteDrag {
    screenX: number;
    screenY: number;
    names: string[];
    sessionId: string;
    sourcePath: string;
    clientTransferId: string;
}
