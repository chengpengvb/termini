export interface RespSftpFileCopy {
    names: string[];
    sourcePath: string;
    sideSessionId: string | null;
    sourceClientTransferId: string;
}

export interface RespSftpFilePaste {
    targetPath: string;
    sideSessionId: string | null;
    targetClientTransferId: string;
}
