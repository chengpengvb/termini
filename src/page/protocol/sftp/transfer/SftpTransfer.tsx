import {Box} from '@mui/material';
import {ReqDevice} from '@/common/model/req/ReqDevice.ts';
import {DEVICE_CONNECTION_TYPE} from '@/common/model/Device.ts';
import {SftpHandle} from "@/page/common/dto/sftp/SftpHandle.ts";
import {useSftpService} from "@/common/context/useAppHooks.tsx";
import {Body} from '@/page/protocol/sftp/transfer/part/Body.tsx';
import {Head} from '@/page/protocol/sftp/transfer/part/Head.tsx';
import {Path} from '@/page/protocol/sftp/transfer/part/Path.tsx';
import {RESP_EVENT_SFTP} from "@/common/model/resp/event/RespEventSftp.ts";
import {BodyConflict} from '@/page/protocol/sftp/transfer/part/body/BodyConflict.tsx';
import {Dispatch, RefObject, SetStateAction, useEffect, useRef, useState} from "react";
import {BodyDragCover} from '@/page/protocol/sftp/transfer/part/body/BodyDragCover.tsx';
import {RespSftpEntries, RespSftpTransferConflict} from "@/common/model/resp/RespSftp.ts";
import {BodyEntriesHandle, BodyEntriesVirtualHandle} from '@/page/common/dto/sftp/BodyEntriesHandle.ts';

export interface SftpTransferProps {
    protocol: number;
    sessionId: string;
    showHidden: boolean;
    reqDevice: ReqDevice;
    dragCoverVisible: boolean;
    pathBackEnabled: boolean;
    onIdentityClick: () => void;
    sideRef?: RefObject<SftpHandle | null>;
    panelRef: RefObject<SftpHandle | null>;
    setShowHidden: Dispatch<SetStateAction<boolean>>;
    dialogContainerRef?: RefObject<HTMLElement | null>;
    virtualRef: RefObject<BodyEntriesVirtualHandle | null>;
    entriesHandleRef: RefObject<BodyEntriesHandle | null>;
}

export function SftpTransfer(props: SftpTransferProps) {
    const sftpService = useSftpService();
    const resetDeviceIdRef = useRef<string | null>(null);
    const [loading, setLoading] = useState<boolean>(true);
    const [filterResetToken, setFilterResetToken] = useState(0);
    const lastListedPathRef = useRef<string | undefined>(undefined);
    const [sftpEntries, setSftpEntries] = useState<RespSftpEntries | null>(null);
    const [conflict, setConflict] = useState<RespSftpTransferConflict | null>(null);
    const {reqDevice, sessionId, sideRef, panelRef, virtualRef, entriesHandleRef, dragCoverVisible, showHidden, setShowHidden, dialogContainerRef, onIdentityClick, pathBackEnabled, protocol} = props;
    const localPane = reqDevice.device.type === DEVICE_CONNECTION_TYPE.LOCAL;

    const handleTransferConflict = (data: any) => {
        setConflict(data as RespSftpTransferConflict);
    }

    const handleConflictAnswer = (code: number, requestId: string): void => {
        sftpService.handleTransferAnswer(sessionId, {data: {code, requestId}});
        setConflict(null);
    }

    const handleListDir = (data: any) => {
        setSftpEntries(data as RespSftpEntries);
        setLoading(false);
        const currentPath = sftpService.getCurrentPath(sessionId);
        if (currentPath !== lastListedPathRef.current) {
            lastListedPathRef.current = currentPath;
            setFilterResetToken(token => token + 1);
        }
    }

    const handleChmodded = () => {
        const currentPath = sftpService.getCurrentPath(sessionId);
        if (!currentPath) {
            return
        }
        sftpService.handleListDir(sessionId, currentPath);
    }

    const handleEvent = (arg: { type: number; data: any }) => {
        switch (arg.type) {
            case RESP_EVENT_SFTP.ON_LIST_DIR:
                handleListDir(arg.data);
                break;
            case RESP_EVENT_SFTP.ON_CHMODDED: {
                handleChmodded();
                break;
            }
            case RESP_EVENT_SFTP.ON_TRANSFER_CONFLICT: {
                handleTransferConflict(arg.data);
                break;
            }
            default:
                break;
        }
    };

    useEffect(() => {
        const off = sftpService.addEventListener(sessionId, arg => {
            handleEvent(arg);
        });
        return () => {
            off();
        };
    }, [sessionId, sftpService]);

    useEffect(() => {
        if (resetDeviceIdRef.current === reqDevice.device.id) {
            return;
        }
        resetDeviceIdRef.current = reqDevice.device.id;
        setLoading(true);
        setConflict(null);
        setSftpEntries(null);
    }, [reqDevice.device.id]);

    return (
        <Box sx={{display: 'flex', flexDirection: 'column', height: '100%', minHeight: 0}}>
            <Head reqDevice={reqDevice} sessionId={sessionId} panelRef={panelRef} filterResetToken={filterResetToken} onIdentityClick={onIdentityClick}/>
            <Path deviceId={reqDevice.device.id} sessionId={sessionId} setLoading={setLoading} entriesHandleRef={entriesHandleRef} pathBackEnabled={pathBackEnabled} localPane={localPane}/>
            <Box sx={{flex: 1, minHeight: 0, position: 'relative', display: 'flex', flexDirection: 'column'}}>
                <Body loading={loading} setLoading={setLoading} sideRef={sideRef} reqDevice={reqDevice} sessionId={sessionId} sftpEntries={sftpEntries} setSftpEntries={setSftpEntries} virtualRef={virtualRef} entriesHandleRef={entriesHandleRef} showHidden={showHidden} setShowHidden={setShowHidden}
                      dialogContainerRef={dialogContainerRef} protocol={protocol}/>
                <BodyDragCover visible={dragCoverVisible}/>
                {conflict ? <BodyConflict conflict={conflict} dialogContainerRef={dialogContainerRef} onAnswer={handleConflictAnswer}/> : null}
            </Box>
        </Box>
    );
}
