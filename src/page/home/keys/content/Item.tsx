import {useState} from 'react';
import type {MouseEvent} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {Box, ButtonBase, Typography} from '@mui/material';
import {Delete, Edit, MoreVert} from '@mui/icons-material';
import {isNil, isNotEmpty} from "@/common/util/StringUtil.ts";
import type {ConfigKeyPair} from '@/common/model/ConfigKeyPair.ts';
import {AlertDialog} from "@/common/components/dialog/AlertDialog.tsx";
import {useConfigKeyPairService} from '@/common/context/useAppHooks.tsx';
import {useLongPressMenu} from '@/page/common/hooks/useLongPressMenu.ts';
import {ConfirmDialog} from "@/common/components/dialog/ConfirmDialog.tsx";
import {CommonMenu, CommonMenuPosition} from "@/common/components/menu/CommonMenu.tsx";
import {resolveKeyPairAvatarLetter, resolveKeyPairAvatarStyle} from '@/page/common/util/keyPairUtil.ts';

export function Item(props: { configKeyPair: ConfigKeyPair, setConfigKeyPair: (value: ConfigKeyPair) => void, setDialogOpen: (open: boolean) => void }) {
    const {t, tf} = useI18n();
    const MENU_ITEM = {EDIT: 1, DELETE: 2,} as const;
    const configKeyPairService = useConfigKeyPairService();
    const [showAlert, setShowAlert] = useState<boolean>(false);
    const {configKeyPair, setConfigKeyPair, setDialogOpen} = props;
    const [confirmDeleteOpen, setConfirmDeleteOpen] = useState<boolean>(false);
    const [menuPosition, setMenuPosition] = useState<CommonMenuPosition | null>(null);
    const longPress = useLongPressMenu((clientX, clientY) => {
        setMenuPosition({top: clientY, left: clientX});
    });

    const resolveKeyPairStatusText = (value: ConfigKeyPair): string => {
        const hasPublic = value.public_key != null && value.public_key.length > 0;
        const hasPrivate = value.private_key != null && value.private_key.length > 0;
        if (!hasPublic && !hasPrivate) {
            return t('home.keys.status.unconfigured');
        }
        if (hasPublic && hasPrivate) {
            return t('home.keys.status.both');
        }
        if (hasPrivate) {
            return t('home.keys.status.privateOnly');
        }
        return t('home.keys.status.publicOnly');
    };

    const handleEdit = async (): Promise<void> => {
        if (isNil(configKeyPair.id)) {
            return;
        }
        const fresh = await configKeyPairService.selectById(configKeyPair.id);
        if (!fresh) {
            setShowAlert(true);
            return;
        }
        setConfigKeyPair(fresh);
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
        if (isNil(configKeyPair.id)) {
            return;
        }
        await configKeyPairService.deleteById(configKeyPair.id);
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
        if (!isNotEmpty(configKeyPair.id)) {
            return;
        }
        setConfirmDeleteOpen(true);
    };

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
                <Box className="tm-device-item-icon has-accent" style={resolveKeyPairAvatarStyle(configKeyPair.name)} aria-hidden>
                    <span className="tm-device-item-icon-letter">{resolveKeyPairAvatarLetter(configKeyPair.name)}</span>
                </Box>
                <Box className="tm-device-item-info">
                    <Typography className="tm-device-item-name tm-ellipsis">{configKeyPair.name}</Typography>
                    <Typography className="tm-device-item-ip tm-ellipsis">{resolveKeyPairStatusText(configKeyPair)}</Typography>
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
            <ConfirmDialog open={confirmDeleteOpen} title={t('home.keys.deleteTitle')} message={tf('home.keys.deleteMessage', {name: configKeyPair.name!})} confirmLabel={t('common.delete')} danger onCancel={handleCancelDelete} onConfirm={() => { void handleConfirmDelete(); }}/>
        </>
    );
}
