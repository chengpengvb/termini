import {RespSftpBase} from "@/common/model/resp/RespSftp.ts";
import {SftpOpenEvent} from "@/common/model/req/event/WsSftp.ts";

export interface SftpMeta {
    currentPath?: string,
    clientTransferId?: string,
    respSftpOpen?: RespSftpBase,
    sftpOpenEvent?: SftpOpenEvent,
}