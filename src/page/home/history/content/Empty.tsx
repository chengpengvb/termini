import {History} from '@mui/icons-material';
import {Box, Typography} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';

export function Empty(props: {onTitleClick: () => void}) {
    const {t} = useI18n();

    return (
        <Box className="tm-empty-state">
            <Box className="tm-empty-state-card">
                <Box className="tm-empty-state-icon">
                    <History className="tm-empty-state-icon-svg"/>
                </Box>
                <Typography className="tm-empty-state-title" onClick={props.onTitleClick}>{t('home.history.emptyTitle')}</Typography>
                <Typography className="tm-empty-state-message">{t('home.history.emptyMessage')}</Typography>
            </Box>
        </Box>
    );
}
