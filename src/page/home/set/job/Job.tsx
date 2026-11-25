import {Box, Typography} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {CollectSection} from '@/page/home/set/job/CollectSection.tsx';
import {SshPoolSection} from '@/page/home/set/job/SshPoolSection.tsx';

export function Job() {
    const {t} = useI18n();

    return (
        <Box className="tm-set-content">
            <Box className="tm-set-content-header">
                <Typography className="tm-set-content-title">{t('set.job.title')}</Typography>
            </Box>
            <CollectSection/>
            <SshPoolSection/>
        </Box>
    );
}
