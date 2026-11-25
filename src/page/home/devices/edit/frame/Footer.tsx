import React, {useState} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {KeyboardArrowDown} from '@mui/icons-material';
import {usePlatformMobile} from '@/common/context/usePlatformMobile';
import {ActionSheet} from '@/page/common/components/dialog/ActionSheet.tsx';
import {Box, Button, Menu, MenuItem, MenuList, Typography} from '@mui/material';
import {DEVICE_EDIT_ACTION_CONNECT, DEVICE_EDIT_ACTION_SAVE} from '@/common/model/dto/DeviceEditAction.ts';

export function Footer(props: { disable: boolean; error?: string | null; selectedAction: number; onSelectedActionChange: (action: number) => void; onCancel: () => void; onSave: () => void; onConnect: () => void }) {
    const {t} = useI18n();
    const mobileViewport = usePlatformMobile();
    const [menuOpen, setMenuOpen] = useState(false);
    const [menuAnchor, setMenuAnchor] = useState<HTMLElement | null>(null);
    const {disable, error, selectedAction, onSelectedActionChange, onCancel, onSave, onConnect} = props;
    const open = mobileViewport ? menuOpen : Boolean(menuAnchor);

    const resolveSelectedLabel = (): string => {
        if (selectedAction === DEVICE_EDIT_ACTION_SAVE) {
            return t('common.save');
        }
        return t('home.common.saveAndConnect');
    };

    const handleCloseMenu = (): void => {
        setMenuOpen(false);
        setMenuAnchor(null);
    };

    const handleSelectAction = (action: number): void => {
        onSelectedActionChange(action);
        handleCloseMenu();
    };

    const handleRun = (): void => {
        if (selectedAction === DEVICE_EDIT_ACTION_SAVE) {
            onSave();
            return;
        }
        onConnect();
    };

    const handleOpenMenu = (event: React.MouseEvent<HTMLElement>): void => {
        event.stopPropagation();
        if (mobileViewport) {
            setMenuOpen(true);
            return;
        }
        setMenuAnchor(event.currentTarget);
    };

    const menuItems = [
        <MenuItem key="save" className={mobileViewport ? (selectedAction === DEVICE_EDIT_ACTION_SAVE ? 'tm-action-sheet-item is-selected' : 'tm-action-sheet-item') : 'tm-menu-item'} onClick={() => handleSelectAction(DEVICE_EDIT_ACTION_SAVE)} sx={mobileViewport ? undefined : {minHeight: 36}}>{t('common.save')}</MenuItem>,
        <MenuItem key="connect" className={mobileViewport ? (selectedAction === DEVICE_EDIT_ACTION_CONNECT ? 'tm-action-sheet-item is-selected' : 'tm-action-sheet-item') : 'tm-menu-item'} onClick={() => handleSelectAction(DEVICE_EDIT_ACTION_CONNECT)} sx={mobileViewport ? undefined : {minHeight: 36}}>{t('home.common.saveAndConnect')}</MenuItem>,
    ];

    return (
        <Box className="tm-edit-dialog-footer">
            <Button onClick={onCancel} className="tm-btn tm-btn-secondary">{t('common.cancel')}</Button>
            <Box className="tm-edit-dialog-footer-spacer">
                {error != null ? (
                    <Typography className="tm-confirm-dialog-error" title={error}>{error}</Typography>
                ) : null}
            </Box>
            <Box className={open ? 'tm-edit-split-btn is-open' : 'tm-edit-split-btn'}>
                <Button disabled={disable} onClick={handleRun} className="tm-btn tm-btn-primary tm-edit-split-btn-main">
                    {resolveSelectedLabel()}
                </Button>
                <Button disabled={disable} onClick={handleOpenMenu} className="tm-btn tm-btn-primary tm-edit-split-btn-more" aria-label={t('home.devices.footerSaveMode')}>
                    <KeyboardArrowDown/>
                </Button>
            </Box>
            {mobileViewport ? (
                <ActionSheet open={open} onClose={handleCloseMenu}>
                    <MenuList disablePadding className="tm-action-sheet-list">
                        {menuItems}
                    </MenuList>
                </ActionSheet>
            ) : (
                <Menu open={open} anchorEl={menuAnchor} onClose={handleCloseMenu} anchorOrigin={{vertical: 'top', horizontal: 'right'}} transformOrigin={{vertical: 'bottom', horizontal: 'right'}}
                      slotProps={{paper: {className: 'tm-menu-paper', sx: {zIndex: 1600, mt: '-4px'}}, list: {className: 'tm-menu-list'}}}>
                    {menuItems}
                </Menu>
            )}
        </Box>
    );
}
