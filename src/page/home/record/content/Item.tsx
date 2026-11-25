import {useState} from 'react';
import type {MouseEvent} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {Box, ButtonBase, Typography} from '@mui/material';
import {Delete, MoreHoriz, PlayArrow} from '@mui/icons-material';
import type {SessionRecord} from '@/common/model/SessionRecord.ts';
import {useLongPressMenu} from '@/page/common/hooks/useLongPressMenu.ts';
import {useSessionRecordService} from '@/common/context/useAppHooks.tsx';
import {ConfirmDialog} from '@/common/components/dialog/ConfirmDialog.tsx';
import {CommonMenu, type CommonMenuPosition} from '@/common/components/menu/CommonMenu.tsx';
import {formatRecordDuration, formatRecordSize, formatRecordTime} from '@/page/common/util/recordFormat.ts';

const MENU_ITEM = {PLAY: 1, DELETE: 2} as const;

export function Item(props: {record: SessionRecord; onPlay: (value: SessionRecord) => void; onDeleted: () => void}) {
    const {t, tf} = useI18n();
    const {record, onPlay, onDeleted} = props;
    const sessionRecordService = useSessionRecordService();
    const [confirmDeleteOpen, setConfirmDeleteOpen] = useState<boolean>(false);
    const [menuPosition, setMenuPosition] = useState<CommonMenuPosition | null>(null);
    const longPress = useLongPressMenu((clientX, clientY) => {
        setMenuPosition({top: clientY, left: clientX});
    });

    function handlePlay(): void {
        onPlay(record);
    }

    function handleMenuClick(event: MouseEvent<HTMLElement>): void {
        event.stopPropagation();
        setMenuPosition({top: event.clientY, left: event.clientX});
    }

    function handleClick(event: MouseEvent<HTMLElement>): void {
        if (longPress.consumeIfFired()) {
            event.preventDefault();
            event.stopPropagation();
            return;
        }
        handlePlay();
    }

    function handleCloseMenu(): void {
        setMenuPosition(null);
    }

    function handleDelete(): void {
        setConfirmDeleteOpen(true);
    }

    function handleCancelDelete(): void {
        setConfirmDeleteOpen(false);
    }

    async function handleConfirmDelete(): Promise<void> {
        setConfirmDeleteOpen(false);
        await sessionRecordService.deleteById(record.id);
        onDeleted();
    }

    return (
        <>
            <Box
                role="button"
                tabIndex={0}
                className="tm-agent-list-row"
                onClick={handleClick}
                onContextMenu={longPress.handleContextMenu}
                onPointerDown={longPress.handlePointerDown}
                onPointerMove={longPress.handlePointerMove}
                onPointerUp={longPress.handlePointerEnd}
                onPointerCancel={longPress.handlePointerEnd}
                onPointerLeave={longPress.handlePointerEnd}
            >
                <Box className="tm-agent-list-row-icon" aria-hidden>
                    <PlayArrow sx={{fontSize: 18}}/>
                </Box>
                <Box className="tm-agent-list-row-main">
                    <Typography className="tm-agent-list-row-name">{record.device_name} · {record.protocol.toUpperCase()}</Typography>
                    <Typography className="tm-agent-list-row-desc">{formatRecordTime(record.started_at)} · {formatRecordDuration(record)} · {formatRecordSize(record.size_bytes)}</Typography>
                </Box>
                <Box className="tm-agent-list-row-actions">
                    <ButtonBase disableRipple className="tm-agent-list-row-btn" aria-label={t('home.common.more')} onClick={handleMenuClick}>
                        <MoreHoriz sx={{fontSize: 18}}/>
                    </ButtonBase>
                </Box>
            </Box>
            <CommonMenu
                position={menuPosition}
                onClose={handleCloseMenu}
                items={[
                    {key: MENU_ITEM.PLAY, text: t('home.record.play'), icon: <PlayArrow/>, onClick: handlePlay},
                    {key: MENU_ITEM.DELETE, text: t('common.delete'), icon: <Delete/>, danger: true, onClick: handleDelete},
                ]}
            />
            <ConfirmDialog
                open={confirmDeleteOpen}
                title={t('home.record.deleteTitle')}
                message={tf('home.record.deleteMessage', {name: record.device_name})}
                confirmLabel={t('common.delete')}
                danger
                onCancel={handleCancelDelete}
                onConfirm={() => {
                    void handleConfirmDelete();
                }}
            />
        </>
    );
}
