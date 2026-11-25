import type {MouseEvent} from 'react';
import {useEffect, useState} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import type {Device} from '@/common/model/Device.ts';
import {Box, ButtonBase, Typography} from '@mui/material';
import {Delete, Edit, MoreVert} from '@mui/icons-material';
import {FORWARD_KIND} from '@/common/model/PortForward.ts';
import {isNil, isNotEmpty} from "@/common/util/StringUtil.ts";
import type {PortForward} from '@/common/model/PortForward.ts';
import {AlertDialog} from "@/common/components/dialog/AlertDialog.tsx";
import {useLongPressMenu} from '@/page/common/hooks/useLongPressMenu.ts';
import {ConfirmDialog} from "@/common/components/dialog/ConfirmDialog.tsx";
import {resolveForwardStatusColor} from "@/page/common/util/forwardUtil.ts";
import {CommonMenu, CommonMenuPosition} from "@/common/components/menu/CommonMenu.tsx";
import {useDeviceService, usePortForwardService} from '@/common/context/useAppHooks.tsx';
import {resolveDeviceOsIcon, resolveDeviceOsIconChipClassName, resolveDeviceOsIconChipStyle} from '@/page/common/util/deviceUtil.ts';

export function Item(props: { portForward: PortForward, setPortForward: (value: PortForward) => void, setDialogOpen: (open: boolean) => void }) {
    const {t, tf} = useI18n();
    const deviceService = useDeviceService();
    const MENU_ITEM = {EDIT: 1, DELETE: 2,} as const;
    const portForwardService = usePortForwardService();
    const [showAlert, setShowAlert] = useState<boolean>(false);
    const {portForward, setPortForward, setDialogOpen} = props;
    const [confirmDeleteOpen, setConfirmDeleteOpen] = useState<boolean>(false);
    const [menuPosition, setMenuPosition] = useState<CommonMenuPosition | null>(null);
    const [device, setDevice] = useState<Device | null>(() => isNotEmpty(portForward.device_id) ? deviceService.getCachedDevice(portForward.device_id) : null);
    const longPress = useLongPressMenu((clientX, clientY) => {
        setMenuPosition({top: clientY, left: clientX});
    });

    const resolveForwardSummaryLabel = (value: PortForward): string => {
        const kind = t(`home.forward.kind.${value.kind}`);
        const bind = `${value.bind_address}:${value.bind_port}`;
        if (value.kind === FORWARD_KIND.DYNAMIC) {
            return tf('home.forward.summaryDynamic', {kind, bind});
        }
        const target = `${value.target_host}:${value.target_port}`;
        if (value.kind === FORWARD_KIND.REMOTE) {
            return tf('home.forward.summaryRemote', {kind, bind, target});
        }
        return tf('home.forward.summaryLocal', {kind, bind, target});
    };

    const handleEdit = async (): Promise<void> => {
        if (isNil(portForward.id)) {
            return;
        }
        const fresh = await portForwardService.selectById(portForward.id);
        if (!fresh) {
            setShowAlert(true);
            return;
        }
        setPortForward(fresh);
        setDialogOpen(true);
    };

    const handleMoreClick = (e: MouseEvent<HTMLElement>): void => {
        e.stopPropagation();
        const rect = e.currentTarget.getBoundingClientRect();
        setMenuPosition({top: rect.top, left: rect.right});
    };

    const handleCancelDelete = (): void => {
        setConfirmDeleteOpen(false);
    };

    const handleConfirmDelete = async (): Promise<void> => {
        setConfirmDeleteOpen(false);
        if (isNil(portForward.id)) {
            return;
        }
        await portForwardService.deleteById(portForward.id);
    };

    const handleClick = (e: MouseEvent<HTMLButtonElement>): void => {
        if (longPress.consumeIfFired()) {
            e.preventDefault();
            e.stopPropagation();
            return;
        }
        void handleEdit();
    };

    const handleCloseMenu = (): void => {
        setMenuPosition(null);
    };

    const handleDelete = (): void => {
        if (!isNotEmpty(portForward.id)) {
            return;
        }
        setConfirmDeleteOpen(true);
    };

    useEffect(() => {
        if (!isNotEmpty(portForward.device_id)) {
            setDevice(null);
            return;
        }
        const deviceId = portForward.device_id;
        let alive = true;
        deviceService.ensureDeviceCache().then(() => {
            if (alive) {
                setDevice(deviceService.getCachedDevice(deviceId));
            }
        });
        return () => {
            alive = false;
        };
    }, [deviceService, portForward.device_id]);

    return (
        <>
            <ButtonBase
                className="tm-device-item"
                onClick={handleClick}
                onContextMenu={longPress.handleContextMenu}
                onPointerDown={longPress.handlePointerDown}
                onPointerMove={longPress.handlePointerMove}
                onPointerUp={longPress.handlePointerEnd}
                onPointerCancel={longPress.handlePointerEnd}
                onPointerLeave={longPress.handlePointerEnd}
            >
                {device !== null && (
                    <Box className={resolveDeviceOsIconChipClassName(device.os_type)} style={resolveDeviceOsIconChipStyle(device.os_type)}>
                        {resolveDeviceOsIcon(device.os_type, 'tm-device-item-os-icon')}
                    </Box>
                )}
                <Box className="tm-device-item-info">
                    <Box sx={{display: 'flex', alignItems: 'center', gap: 0.75, minWidth: 0}}>
                        <Box sx={{width: 7, height: 7, borderRadius: '50%', flexShrink: 0, bgcolor: resolveForwardStatusColor(portForward.status)}}/>
                        <Typography className="tm-device-item-name tm-ellipsis">{portForward.name}</Typography>
                    </Box>
                    <Typography className="tm-device-item-ip tm-ellipsis">{resolveForwardSummaryLabel(portForward)}</Typography>
                </Box>
                <Box className="tm-device-item-actions">
                    <Box className="tm-device-item-action-btn" onClick={handleMoreClick} role="button" aria-label={t('home.common.more')}>
                        <MoreVert/>
                    </Box>
                </Box>
            </ButtonBase>

            <AlertDialog duration={1500} open={showAlert} handleClose={() => setShowAlert(false)} message={t('home.common.dataLoadFailed')} type="error"/>

            <CommonMenu
                position={menuPosition}
                onClose={handleCloseMenu}
                items={[
                    {key: MENU_ITEM.EDIT, text: t('common.edit'), icon: <Edit/>, onClick: () => { void handleEdit(); }},
                    {key: MENU_ITEM.DELETE, text: t('common.delete'), icon: <Delete/>, danger: true, onClick: handleDelete},
                ]}
            />
            <ConfirmDialog open={confirmDeleteOpen} title={t('home.forward.deleteTitle')} message={tf('home.forward.deleteMessage', {name: portForward.name!})} confirmLabel={t('common.delete')} danger onCancel={handleCancelDelete} onConfirm={() => { void handleConfirmDelete(); }}/>
        </>
    );
}
