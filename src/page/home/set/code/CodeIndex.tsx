import {Box, Typography} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {CodeIndexSection} from '@/page/home/set/code/CodeIndexSection.tsx';

export function CodeIndex() {
    const {t} = useI18n();

    return (
        <Box className="tm-set-content">
            <Box className="tm-set-content-header">
                <Typography className="tm-set-content-title">{t('set.codeIndex.title')}</Typography>
            </Box>
            <CodeIndexSection/>
        </Box>
    );
}
