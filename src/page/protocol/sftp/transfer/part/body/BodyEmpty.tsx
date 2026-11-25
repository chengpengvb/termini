import {useI18n} from '@/common/i18n/useI18n.ts';
import {CircularProgress, Typography} from '@mui/material';

export function BodyEmpty() {
    const {t} = useI18n();
    return (
        <>
            <CircularProgress size={28} sx={{color: 'var(--tm-icon-accent)'}} />
            <Typography className="tm-home-loading-text">{t('home.common.loading')}</Typography>
        </>
    );
}
