import {Box} from '@mui/material';
import {Dispatch, RefObject, SetStateAction} from "react";
import {ReqDevice} from '@/common/model/req/ReqDevice.ts';
import {DEVICE_CONNECTION_TYPE} from '@/common/model/Device.ts';
import {RespSftpEntries} from "@/common/model/resp/RespSftp.ts";
import {SftpHandle} from "@/page/common/dto/sftp/SftpHandle.ts";
import {BodyEmpty} from '@/page/protocol/sftp/transfer/part/body/BodyEmpty.tsx';
import {BodyEntries} from '@/page/protocol/sftp/transfer/part/body/BodyEntries.tsx';
import {BodyEntriesHandle, BodyEntriesVirtualHandle} from '@/page/common/dto/sftp/BodyEntriesHandle.ts';

export interface BodyProps {
    protocol: number;
    loading: boolean;
    sessionId: string;
    showHidden: boolean;
    reqDevice: ReqDevice | null;
    sftpEntries: RespSftpEntries | null;
    sideRef?: RefObject<SftpHandle | null>;
    setLoading: Dispatch<SetStateAction<boolean>>;
    setShowHidden: Dispatch<SetStateAction<boolean>>;
    dialogContainerRef?: RefObject<HTMLElement | null>;
    entriesHandleRef: RefObject<BodyEntriesHandle | null>;
    virtualRef: RefObject<BodyEntriesVirtualHandle | null>;
    setSftpEntries: Dispatch<SetStateAction<RespSftpEntries | null>>;
}

export function Body(props: BodyProps) {
    const {sessionId, sftpEntries, loading, setLoading, sideRef, virtualRef, showHidden, setShowHidden, dialogContainerRef, entriesHandleRef, reqDevice, protocol} = props;
    const localPane = reqDevice !== null && reqDevice.device.type === DEVICE_CONNECTION_TYPE.LOCAL;

    function resolveEntriesClassName(): string {
        if (loading) {
            return 'tm-sftp-body-entries tm-sftp-body-entries-loading';
        }
        return 'tm-sftp-body-entries';
    }

    return (
        <Box className="tm-sftp-body">
            {sftpEntries ? (
                <Box className={resolveEntriesClassName()}>
                    <BodyEntries setLoading={setLoading} sideRef={sideRef} sessionId={sessionId} sftpEntries={sftpEntries} virtualRef={virtualRef} entriesHandleRef={entriesHandleRef} showHidden={showHidden} setShowHidden={setShowHidden} dialogContainerRef={dialogContainerRef} localPane={localPane} protocol={protocol}/>
                </Box>
            ) : null}
            {loading ? (
                <Box className="tm-sftp-body-loading">
                    <BodyEmpty/>
                </Box>
            ) : null}
        </Box>
    );
}
