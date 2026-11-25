import type {RefObject} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {HOME_MENU} from '../../common/model/HomeMenu';
import {Box, ButtonBase, Typography} from '@mui/material';
import {NavMenuItem} from '@/page/common/components/nav/NavMenuItem.tsx';
import {AssignmentOutlined, ChevronLeft, DataObject, Devices, OndemandVideo, Settings, SwapHoriz, VpnKey} from '@mui/icons-material';

export interface SidebarProps {
    width?: number;
    currentMenu: number;
    mobileOpen: boolean;
    mobileViewport: boolean;
    onMobileClose: () => void;
    onMenuChange: (menu: number) => void;
    rootRef?: RefObject<HTMLDivElement | null>;
}

const MENU_ITEMS = [
    {menu: HOME_MENU.DEVICES, labelKey: 'home.menu.devices', icon: <Devices className="tm-nav-item-icon"/>},
    {menu: HOME_MENU.KEYS, labelKey: 'home.menu.keys', icon: <VpnKey className="tm-nav-item-icon"/>},
    {menu: HOME_MENU.FORWARD, labelKey: 'home.menu.forward', icon: <SwapHoriz className="tm-nav-item-icon"/>},
    {menu: HOME_MENU.SNIPPETS, labelKey: 'home.menu.snippets', icon: <DataObject className="tm-nav-item-icon"/>},
    {menu: HOME_MENU.RECORDS, labelKey: 'home.menu.records', icon: <OndemandVideo className="tm-nav-item-icon"/>},
    {menu: HOME_MENU.JOBS, labelKey: 'home.menu.jobs', icon: <AssignmentOutlined className="tm-nav-item-icon"/>},
    {menu: HOME_MENU.SET, labelKey: 'home.menu.settings', icon: <Settings className="tm-nav-item-icon"/>},
] as const;

export function Sidebar(props: SidebarProps) {
    const {width, currentMenu, onMenuChange, mobileViewport, mobileOpen, onMobileClose, rootRef} = props;
    const {t} = useI18n();

    const renderMenu = () => {
        return (
            <Box className="tm-nav-menu">
                {MENU_ITEMS.map(item => (
                    <NavMenuItem key={item.menu} navMenu={item.menu} label={t(item.labelKey)} selected={currentMenu === item.menu} icon={item.icon} onClick={() => onMenuChange(item.menu)}/>
                ))}
            </Box>
        );
    };

    if (mobileViewport) {
        if (!mobileOpen) {
            return null;
        }
        return (
            <Box className="tm-home-sidebar-mobile-root">
                <Box onClick={onMobileClose} className="tm-home-sidebar-mobile-backdrop"/>
                <Box className="tm-home-sidebar-mobile-panel">
                    <Box className="tm-home-sidebar-mobile-header">
                        <Typography className="tm-home-sidebar-mobile-title">Termini</Typography>
                        <ButtonBase onClick={onMobileClose} className="tm-home-sidebar-mobile-close">
                            <ChevronLeft className="tm-home-sidebar-mobile-close-icon"/>
                        </ButtonBase>
                    </Box>
                    {renderMenu()}
                </Box>
            </Box>
        );
    }

    return (
        <Box ref={rootRef} className="tm-home-sidebar" style={width ? {width} : undefined}>
            {renderMenu()}
        </Box>
    );
}
