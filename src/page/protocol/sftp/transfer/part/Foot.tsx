import {Box} from '@mui/material';
import {SftpHandle} from '@/page/common/dto/sftp/SftpHandle.ts';
import {useSftpService} from '@/common/context/useAppHooks.tsx';
import {RefObject, useEffect, useMemo, useRef, useState} from 'react';
import {RESP_EVENT_SFTP} from '@/common/model/resp/event/RespEventSftp.ts';
import {DeleteProgress} from '@/page/protocol/sftp/transfer/part/foot/DeleteProgress.tsx';
import {TransferProgress} from '@/page/protocol/sftp/transfer/part/foot/TransferProgress.tsx';
import {RespSftpBase, RespSftpDeleteProgress, RespSftpTfProgress} from '@/common/model/resp/RespSftp.ts';

export interface FootProps {
    leftRef?: RefObject<SftpHandle | null>;
    rightRef?: RefObject<SftpHandle | null>;
}

interface TransferProgressItem {
    sessionId: string;
    progress: RespSftpTfProgress;
}

interface DeleteProgressItem {
    sessionId: string;
    progress: RespSftpDeleteProgress;
}

export function Foot(props: FootProps) {
    const {leftRef, rightRef} = props;
    const sftpService = useSftpService();
    const deleteSessionByRespIdRef = useRef<Map<string, string>>(new Map());
    const transferSessionByRespIdRef = useRef<Map<string, string>>(new Map());
    const [deleteProgressByRespId, setDeleteProgressByRespId] = useState<Map<string, DeleteProgressItem>>(new Map());
    const [transferProgressByRespId, setTransferProgressByRespId] = useState<Map<string, TransferProgressItem>>(new Map());

    const handleTransferEvent = (arg: { type: number; data: any }) => {
        const respSftpBase = arg.data as RespSftpBase;
        const respId = respSftpBase.respId;
        if (!respId) {
            return;
        }
        transferSessionByRespIdRef.current.delete(respId);
        setTransferProgressByRespId(current => {
            const next = new Map(current);
            next.delete(respId);
            return next;
        });
    };

    const handleTransferProgressEvent = (sessionId: string, arg: { type: number; data: any }) => {
        const progress = arg.data as RespSftpTfProgress;
        transferSessionByRespIdRef.current.set(progress.respId, sessionId);
        setTransferProgressByRespId(current => {
            const next = new Map(current);
            next.set(progress.respId, {progress, sessionId});
            return next;
        });
    };

    const handleDeletedEvent = (arg: { type: number; data: any }) => {
        const respSftpBase = arg.data as RespSftpBase;
        const respId = respSftpBase.respId;
        if (!respId) {
            return;
        }
        deleteSessionByRespIdRef.current.delete(respId);
        setDeleteProgressByRespId(current => {
            const next = new Map(current);
            next.delete(respId);
            return next;
        });
    };

    const handleDeleteProgressEvent = (sessionId: string, arg: { type: number; data: any }) => {
        const progress = arg.data as RespSftpDeleteProgress;
        deleteSessionByRespIdRef.current.set(progress.respId, sessionId);
        setDeleteProgressByRespId(current => {
            const next = new Map(current);
            next.set(progress.respId, {progress, sessionId});
            return next;
        });
    };

    const handleEvent = (sessionId: string, arg: { type: number; data: any }) => {
        switch (arg.type) {
            case RESP_EVENT_SFTP.ON_TRANSFER:
                handleTransferEvent(arg);
                break;
            case RESP_EVENT_SFTP.ON_TRANSFER_PROGRESS:
                handleTransferProgressEvent(sessionId, arg);
                break;
            case RESP_EVENT_SFTP.ON_DELETED:
                handleDeletedEvent(arg);
                break;
            case RESP_EVENT_SFTP.ON_DELETE_PROGRESS:
                handleDeleteProgressEvent(sessionId, arg);
                break;
            default:
                break;
        }
    };

    useEffect(() => {
        if (!rightRef || !rightRef.current) {
            return;
        }
        const sessionId = rightRef.current.getSessionId();
        const off = sftpService.addEventListener(sessionId, arg => {
            handleEvent(sessionId, arg);
        });
        return () => {
            off();
        };
    }, [rightRef, sftpService]);

    useEffect(() => {
        if (!leftRef || !leftRef.current) {
            return;
        }
        const sessionId = leftRef.current.getSessionId();
        const off = sftpService.addEventListener(sessionId, arg => {
            handleEvent(sessionId, arg);
        });
        return () => {
            off();
        };
    }, [leftRef, sftpService]);

    const handleDismissTransfer = (respId: string): void => {
        const sessionId = transferSessionByRespIdRef.current.get(respId);
        if (sessionId) {
            sftpService.handleTransferCancel(sessionId, {data: {respId}});
        }
        transferSessionByRespIdRef.current.delete(respId);
        setTransferProgressByRespId(current => {
            const next = new Map(current);
            next.delete(respId);
            return next;
        });
    };

    const handleDismissDelete = (respId: string): void => {
        const sessionId = deleteSessionByRespIdRef.current.get(respId);
        if (sessionId) {
            sftpService.handleDeleteCancel(sessionId, {data: {respId}});
        }
        deleteSessionByRespIdRef.current.delete(respId);
        setDeleteProgressByRespId(current => {
            const next = new Map(current);
            next.delete(respId);
            return next;
        });
    };

    const transferProgressList = useMemo(() => {
        return Array.from(transferProgressByRespId.values());
    }, [transferProgressByRespId]);

    const deleteProgressList = useMemo(() => {
        return Array.from(deleteProgressByRespId.values());
    }, [deleteProgressByRespId]);

    if (transferProgressList.length === 0 && deleteProgressList.length === 0) {
        return null;
    }

    return (
        <Box sx={{position: 'absolute', left: 0, right: 0, bottom: 0, zIndex: 1400, pointerEvents: 'none'}}>
            <Box className="tm-sftp-foot-panel">
                <Box className="tm-sftp-foot-list">
                    {transferProgressList.map(item => (
                        <TransferProgress key={item.progress.respId} progress={item.progress} onClose={() => handleDismissTransfer(item.progress.respId)}/>
                    ))}
                    {deleteProgressList.map(item => (
                        <DeleteProgress key={item.progress.respId} progress={item.progress} onClose={() => handleDismissDelete(item.progress.respId)}/>
                    ))}
                </Box>
            </Box>
        </Box>
    );
}
