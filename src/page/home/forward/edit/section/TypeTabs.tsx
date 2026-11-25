import {useMemo} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {Box, ButtonBase, Typography} from '@mui/material';
import {resolveForwardTypeTabs} from '@/page/common/util/forwardUtil.ts';

export function TypeTabs(props: { value: number; onChange: (kind: number) => void }) {
    const {language} = useI18n();
    const {value, onChange} = props;
    const tabs = useMemo(() => resolveForwardTypeTabs(), [language]);

    return (
        <Box className="tm-job-tabs" sx={{width: '100%', boxSizing: 'border-box'}}>
            {tabs.map(tab => (
                <ButtonBase key={tab.value} disableRipple className={value === tab.value ? 'tm-job-tab is-active' : 'tm-job-tab'} sx={{flex: 1}} onClick={() => onChange(tab.value)}>
                    <Typography component="span">{tab.label}</Typography>
                </ButtonBase>
            ))}
        </Box>
    );
}
