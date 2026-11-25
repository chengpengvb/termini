import type {MouseEvent} from 'react';
import {useEffect, useState} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {Cable, UsbOutlined} from '@mui/icons-material';
import {SerialPage} from '@/page/protocol/serial/SerialPage.tsx';
import type {ConfigSerial} from '@/common/model/ConfigSerial.ts';
import {usePlatformMobile} from '@/common/context/usePlatformMobile';
import {Box, ButtonBase, Menu, MenuItem, MenuList} from '@mui/material';
import {ActionSheet} from '@/page/common/components/dialog/ActionSheet.tsx';
import {SerialConnectDialog, type SerialConnectValue} from './SerialConnectDialog.tsx';
import {useSerialService, useTabManagerService} from '@/common/context/useAppHooks.tsx';

export function SerialAction() {
    const {t} = useI18n();
    const serialService = useSerialService();
    const mobileViewport = usePlatformMobile();
    const [ports, setPorts] = useState<string[]>([]);
    const tabManagerService = useTabManagerService();
    const [open, setOpen] = useState<boolean>(false);
    const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null);
    const [pendingPort, setPendingPort] = useState<string | null>(null);
    const [pendingConfig, setPendingConfig] = useState<ConfigSerial | null>(null);

    const handleToggle = (event: MouseEvent<HTMLElement>): void => {
        setAnchorEl(event.currentTarget);
        serialService.ports().then(setPorts);
        setOpen(prev => !prev);
    };

    const handleClose = (): void => {
        setOpen(false);
    };

    const handlePick = async (port: string): Promise<void> => {
        setOpen(false);
        const existing = await serialService.config(port);
        setPendingConfig(existing);
        setPendingPort(port);
    };

    const handleConnectCancel = (): void => {
        setPendingPort(null);
        setPendingConfig(null);
    };

    const openSerialTab = (port: string): void => {
        tabManagerService.openTab({
            panes: [{
                tabBar: {
                    title: port,
                    canClose: true,
                    icon: <UsbOutlined sx={{fontSize: 14}}/>,
                },
                content: <SerialPage portName={port}/>,
            }],
        });
    };

    const handleConnect = async (value: SerialConnectValue): Promise<void> => {
        if (pendingPort === null) {
            return;
        }
        const port = pendingPort;
        await serialService.saveConfig({...pendingConfig, ...value, port_name: port});
        setPendingPort(null);
        setPendingConfig(null);
        openSerialTab(port);
    };

    const renderItems = (mobile: boolean) => {
        if (ports.length === 0) {
            return [
                <MenuItem key="__empty" disabled dense={!mobile} className={mobile ? 'tm-action-sheet-item' : 'tm-menu-item'}>
                    <Box component="span" className="tm-menu-item-text">
                        {t('home.devices.serialEmpty')}
                    </Box>
                </MenuItem>,
            ];
        }
        return ports.map(port => (
            <MenuItem key={port} dense={!mobile} className={mobile ? 'tm-action-sheet-item' : 'tm-menu-item'} onClick={() => { void handlePick(port); }}>
                <Box component="span" className="tm-menu-item-icon">
                    <UsbOutlined sx={{fontSize: 16}}/>
                </Box>
                <Box component="span" className="tm-menu-item-text">
                    {port}
                </Box>
            </MenuItem>
        ));
    };

    useEffect(() => {
        serialService.ports().then(setPorts);
        return serialService.addPortsChangeListener(setPorts);
    }, [serialService]);

    return (
        <>
            <ButtonBase onClick={handleToggle} className="tm-action-btn tm-serial-action-btn">
                <Cable sx={{fontSize: 16}}/>
                <Box component="span" className="tm-action-btn-label">
                    {t('home.devices.serialAction')}
                </Box>
            </ButtonBase>
            {mobileViewport ? (
                <ActionSheet open={open} onClose={handleClose}>
                    <MenuList disablePadding className="tm-action-sheet-list">
                        {renderItems(true)}
                    </MenuList>
                </ActionSheet>
            ) : (
                <Menu
                    open={open}
                    onClose={handleClose}
                    anchorEl={anchorEl}
                    anchorOrigin={{vertical: 'bottom', horizontal: 'left'}}
                    transformOrigin={{vertical: 'top', horizontal: 'left'}}
                    slotProps={{paper: {className: 'tm-menu-paper', sx: {mt: '2px'}}, list: {className: 'tm-menu-list'}}}
                >
                    {renderItems(false)}
                </Menu>
            )}
            <SerialConnectDialog open={pendingPort !== null} port={pendingPort ?? ''} config={pendingConfig} onCancel={handleConnectCancel} onConnect={value => { void handleConnect(value); }} />
        </>
    );
}
