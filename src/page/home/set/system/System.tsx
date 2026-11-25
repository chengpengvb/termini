import {Box, Typography} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {inTauri} from '@/common/util/TauriUtil.ts';
import {GeneralSection} from '@/page/home/set/system/GeneralSection.tsx';
import {MasterKeySection} from '@/page/home/set/system/MasterKeySection.tsx';

export function System() {
    const {t} = useI18n();

    return (
        <Box className="tm-set-content">
            <Box className="tm-set-content-header">
                <Typography className="tm-set-content-title">{t('set.system.title')}</Typography>
            </Box>
            <GeneralSection/>
            {inTauri() ? <MasterKeySection/> : null}
        </Box>
    );
}
