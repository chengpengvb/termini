import {Box, Button} from "@mui/material";
import {useI18n} from '@/common/i18n/useI18n.ts';
import {usePaneContext} from "@/common/components/tab";
import {type RefObject, useEffect, useState} from "react";
import {ConnectService} from "@/service/ConnectService.ts";
import type {ReqDevice} from "@/common/model/req/ReqDevice.ts";
import {EditPanel} from "@/page/home/devices/edit/EditPanel.tsx";
import {StepHeadHandle} from "@/page/protocol/common/step/StepHead.tsx";
import {StepContextHandle} from "@/page/protocol/common/step/StepContext.tsx";
import {useDeviceEditService, useDeviceService, useTabManagerService} from "@/common/context/useAppHooks.tsx";

export interface BottomBoxProps {
    failed: boolean;
    sessionId: string;
    reqDevice: ReqDevice,
    connectService: ConnectService | undefined;
    stepHeadRef: RefObject<StepHeadHandle | null>;
    stepContextHandle: StepContextHandle | undefined;
}

export function BottomBox(props: BottomBoxProps) {
    const {t} = useI18n();
    const {paneId} = usePaneContext();
    const deviceService = useDeviceService();
    const deviceEditService = useDeviceEditService();
    const tabManagerService = useTabManagerService();
    const [editOpen, setEditOpen] = useState<boolean>(false);
    const [reqDevice, setReqDevice] = useState<ReqDevice>(props.reqDevice);
    const {failed, stepContextHandle, stepHeadRef, connectService, sessionId} = props;
    const [selectedAction, setSelectedAction] = useState<number>(() => deviceEditService.getSelectedAction());

    const onClose = (): void => {
        tabManagerService.closePane(paneId);
    };

    const handleClose = (): void => {
        setEditOpen(false)
    }

    const handleSave = (_value: ReqDevice): void => {
        setEditOpen(false)
    };

    const handleConnect = (): void => {
        setEditOpen(false)
        stepContextHandle?.clear()
        stepHeadRef.current?.clear()
        connectService?.noticeRetry(sessionId)
    }

    const onEdit = (): void => {
        updateReqDevice().then(() => {
            setEditOpen(true);
        });
    }

    const handleSelectedActionChange = (action: number): void => {
        deviceEditService.setSelectedAction(action);
    }

    const onRetry = (): void => {
        stepContextHandle?.clear()
        stepHeadRef.current?.clear()
        connectService?.noticeRetry(sessionId)
    }

    const updateReqDevice = async (): Promise<void> => {
        const loaded = await deviceService.selectWithConfig(reqDevice.device.id);
        if (!loaded) {
            return;
        }
        setReqDevice(loaded);
    }

    useEffect(() => {
        const offAction = deviceEditService.addSelectedActionListener(setSelectedAction);
        return () => {
            offAction();
        };
    }, [deviceEditService]);

    if (!failed) {
        return (
            <Box className="tm-protocol-select-actions" sx={{justifyContent: 'flex-end'}}>
                <Button disableRipple onClick={onClose} className="tm-btn tm-btn-secondary">{t('common.cancel')}</Button>
            </Box>
        );
    }

    return (
        <Box className="tm-protocol-select-actions">
            <Box className="tm-protocol-select-actions-left">
                <Button disableRipple onClick={onClose} className="tm-btn tm-btn-secondary">{t('common.close')}</Button>
                <Button disableRipple onClick={onEdit} className="tm-btn tm-btn-secondary">{t('common.edit')}</Button>
            </Box>
            <Button disableRipple onClick={onRetry} className="tm-btn tm-btn-primary">
                {t('common.retry')}
            </Button>

            <EditPanel open={editOpen} reqDevice={reqDevice} selectedAction={selectedAction} onSelectedActionChange={handleSelectedActionChange} onSave={handleSave} onConnect={handleConnect} onClose={handleClose} onCancel={handleClose}/>

        </Box>
    );
}
