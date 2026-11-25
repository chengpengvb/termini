import {useState, type MouseEvent} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNotEmpty} from "@/common/util/StringUtil.ts";
import {Box, ButtonBase, Typography} from '@mui/material';
import type {ReqDevice} from '@/common/model/req/ReqDevice.ts';
import {Cable, Delete, Edit, MoreVert} from '@mui/icons-material';
import {resolveErrorText} from '@/common/model/resp/ResultModel.ts';
import {AlertDialog} from "@/common/components/dialog/AlertDialog.tsx";
import {useLongPressMenu} from '@/page/common/hooks/useLongPressMenu.ts';
import {ConfirmDialog} from "@/common/components/dialog/ConfirmDialog.tsx";
import {SelectProtocol} from '@/page/protocol/common/select/SelectProtocol.tsx';
import {CommonMenu, CommonMenuPosition} from "@/common/components/menu/CommonMenu.tsx";
import {useDeviceService, useTabManagerService} from '@/common/context/useAppHooks.tsx';
import {resolveDeviceOsIcon, resolveDeviceOsIconChipClassName, resolveDeviceOsIconChipStyle, resolveDeviceTabIcon} from '@/page/common/util/deviceUtil.ts';

const MENU_ITEM = {CONNECT: 0, EDIT: 1, DELETE: 2} as const;

export function Item(props: { reqDevice: ReqDevice, setReqDevice: (reqDevice: ReqDevice) => void, setDrawerVisible: (visible: boolean) => void }) {
    const {t, tf} = useI18n();
    const {device} = props.reqDevice;
    const {reqDevice, setReqDevice} = props;
    const deviceService = useDeviceService();
    const tabManagerService = useTabManagerService();
    const [showAlert, setShowAlert] = useState<boolean>(false);
    const [confirmDeleteOpen, setConfirmDeleteOpen] = useState<boolean>(false);
    const [menuPosition, setMenuPosition] = useState<CommonMenuPosition | null>(null);
    const [alertMessage, setAlertMessage] = useState<string>(t('home.common.dataLoadFailed'));
    const longPress = useLongPressMenu((clientX, clientY) => {
        setMenuPosition({top: clientY, left: clientX});
    });

    const handleEdit = async (): Promise<void> => {
        try {
            const loaded = await deviceService.selectWithConfig(device.id);
            if (!loaded) {
                setAlertMessage(t('home.common.dataLoadFailed'));
                setShowAlert(true);
                return;
            }
            setReqDevice(loaded);
            props.setDrawerVisible(true);
        } catch (err) {
            setAlertMessage(resolveErrorText(err));
            setShowAlert(true);
        }
    };

    const handleCancelDelete = (): void => {
        setConfirmDeleteOpen(false);
    };

    const handleConfirmDelete = async (): Promise<void> => {
        setConfirmDeleteOpen(false);
        await deviceService.delete(device.id);
    };

    const handleConnect = (): void => {
        tabManagerService.openTab({
            panes: [{
                tabBar: {
                    canClose: true,
                    title: device.name,
                    icon: resolveDeviceTabIcon(device.os_type),
                },
                content: <SelectProtocol reqDevice={reqDevice}/>,
            }],
        });
    };

    const handleClick = (e: MouseEvent<HTMLButtonElement>): void => {
        if (longPress.consumeIfFired()) {
            e.preventDefault();
            e.stopPropagation();
            return;
        }
        handleConnect();
    };

    const handleMoreClick = (e: MouseEvent<HTMLElement>): void => {
        e.stopPropagation();
        const rect = e.currentTarget.getBoundingClientRect();
        setMenuPosition({top: rect.top, left: rect.right});
    };

    const handleCloseMenu = (): void => {
        setMenuPosition(null);
    };

    const handleDelete = (): void => {
        if (!isNotEmpty(device.id)) {
            return;
        }
        setConfirmDeleteOpen(true);
    };

    const hasTermCredential = (username: string | null | undefined, password: string | null | undefined): boolean => {
        return isNotEmpty(username) || isNotEmpty(password);
    };

    const resolveProtocolMeta = (): string => {
        const labels: string[] = [];
        const ssh = reqDevice.config_ssh;
        if (hasTermCredential(ssh?.base?.username, ssh?.base?.password) || isNotEmpty(ssh?.config_key_pair_id)) {
            labels.push('ssh');
        }
        const ftp = reqDevice.config_ftp;
        if (hasTermCredential(ftp?.base?.username, ftp?.base?.password)) {
            labels.push('ftp');
        }
        const telnet = reqDevice.config_telnet;
        if (hasTermCredential(telnet?.base?.username, telnet?.base?.password)) {
            labels.push('telnet');
        }
        const rdp = reqDevice.config_rdp;
        if (hasTermCredential(rdp?.base?.username, rdp?.base?.password)) {
            labels.push('rdp');
        }
        if (reqDevice.config_snmp?.enabled === true) {
            labels.push('snmp');
        }
        return labels.join(', ');
    };

    const resolveHostMeta = (): string => {
        const username = reqDevice.config_ssh?.base?.username;
        if (isNotEmpty(username)) {
            return `${username}@${device.host}`;
        }
        return device.host;
    };

    const protocolMeta = resolveProtocolMeta();
    const hostMeta = resolveHostMeta();
    const metaTitle = protocolMeta ? `${protocolMeta} · ${hostMeta}` : hostMeta;

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
                <Box className={resolveDeviceOsIconChipClassName(device.os_type)} style={resolveDeviceOsIconChipStyle(device.os_type)}>
                    {resolveDeviceOsIcon(device.os_type, 'tm-device-item-os-icon')}
                </Box>
                <Box className="tm-device-item-info">
                    <Typography className="tm-device-item-name tm-ellipsis">{device.name}</Typography>
                    <Typography className="tm-device-item-ip tm-ellipsis" title={metaTitle}>
                        {protocolMeta ? <span className="tm-device-item-protocols">{protocolMeta}</span> : null}
                        {protocolMeta ? <span className="tm-device-item-meta-dot">·</span> : null}
                        {hostMeta}
                    </Typography>
                </Box>
                <Box className="tm-device-item-actions">
                    <Box className="tm-device-item-action-btn" onClick={handleMoreClick} role="button" aria-label={t('home.common.more')}>
                        <MoreVert/>
                    </Box>
                </Box>
            </ButtonBase>

            <AlertDialog duration={1500} open={showAlert} handleClose={() => setShowAlert(false)} message={alertMessage} type="error"/>

            <CommonMenu
                position={menuPosition}
                onClose={handleCloseMenu}
                items={[
                    {key: MENU_ITEM.CONNECT, text: t('home.common.connect'), icon: <Cable/>, accent: true, onClick: handleConnect},
                    {
                        key: MENU_ITEM.EDIT, text: t('common.edit'), icon: <Edit/>, onClick: () => {
                            void handleEdit();
                        }
                    },
                    {key: MENU_ITEM.DELETE, text: t('common.delete'), icon: <Delete/>, danger: true, onClick: handleDelete},
                ]}
            />
            <ConfirmDialog open={confirmDeleteOpen} title={t('home.devices.deleteTitle')} message={tf('home.devices.deleteMessage', {name: device.name})} confirmLabel={t('common.delete')} danger onCancel={handleCancelDelete} onConfirm={() => {
                void handleConfirmDelete();
            }}/>
        </>
    );
}
