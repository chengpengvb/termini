import type {ReactNode} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {Box, Button, CircularProgress} from '@mui/material';

export function Footer(props: { disable: boolean; saving?: boolean; saveLabel?: string; onCancel: () => void; onSave: () => void }) {
    const {disable, saving, saveLabel, onCancel, onSave} = props;
    const {t} = useI18n();

    function renderSaveStartIcon(): ReactNode {
        if (saving !== true) {
            return null;
        }
        return <CircularProgress size={14} color="inherit"/>;
    }

    return (
        <Box className="tm-edit-dialog-footer">
            <Button onClick={onCancel} className="tm-btn tm-btn-secondary">{t('common.cancel')}</Button>
            <Box className="tm-edit-dialog-footer-spacer"/>
            <Button disabled={disable} onClick={onSave} startIcon={renderSaveStartIcon()} className="tm-btn tm-btn-primary">
                {saveLabel === undefined ? t('common.save') : saveLabel}
            </Button>
        </Box>
    );
}
