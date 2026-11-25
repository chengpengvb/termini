import {Box, Typography} from '@mui/material';
import {SwapHoriz} from '@mui/icons-material';
import {useI18n} from '@/common/i18n/useI18n.ts';

export function Empty(props: { switchVisible: () => void }) {
    const {switchVisible} = props;
    const {t} = useI18n();

    return (
        <Box className="tm-empty-state">
            <Box className="tm-empty-state-card">
                <Box className="tm-empty-state-icon">
                    <SwapHoriz className="tm-empty-state-icon-svg"/>
                </Box>
                <Typography className="tm-empty-state-title" onClick={switchVisible}>{t('home.forward.emptyTitle')}</Typography>
                <Typography className="tm-empty-state-message">{t('home.forward.emptyMessage')}</Typography>
            </Box>
        </Box>
    );
}
