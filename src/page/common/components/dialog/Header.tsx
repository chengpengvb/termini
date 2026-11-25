import type { ReactNode } from 'react';
import { Close } from '@mui/icons-material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { isNotNil } from '@/common/util/StringUtil.ts';
import { Box, ButtonBase, Typography } from '@mui/material';

export interface EditDialogHeaderProps {
    title: string;
    icon?: ReactNode;
    onClose?: () => void;
}

export function Header(props: EditDialogHeaderProps) {
    const { t } = useI18n();
    const { title, icon, onClose } = props;

    const handleClose = (): void => {
        if (!onClose) {
            return;
        }
        onClose();
    };

    return (
        <Box className="tm-edit-dialog-header" sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', px: 2, py: 1.5, borderBottom: '1px solid var(--tm-border-subtle)' }}>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, minWidth: 0 }}>
                {isNotNil(icon) ? (
                    <Box sx={{ display: 'flex', flexShrink: 0, alignItems: 'center' }}>{icon}</Box>
                ) : null}
                <Typography className="tm-edit-dialog-title" sx={{ fontSize: 14, fontWeight: 700, color: 'var(--tm-text-primary)' }}>{title}</Typography>
            </Box>
            {isNotNil(onClose) ? (
                <ButtonBase className="tm-edit-dialog-close" onClick={handleClose} aria-label={t('common.close')} sx={{ width: 24, height: 24, borderRadius: '6px', color: 'var(--tm-text-tertiary)', '&:hover': { bgcolor: 'var(--tm-bg-selected)', color: 'var(--tm-text-primary)' } }}>
                    <Close sx={{ fontSize: 16 }} />
                </ButtonBase>
            ) : null}
        </Box>
    );
}

