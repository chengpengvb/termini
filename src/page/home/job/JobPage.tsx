import {useEffect, useState} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {DiscoverPage} from './discover/DiscoverPage';
import {OverviewPage} from './overview/OverviewPage';
import {HAPTIC_KIND} from '@/common/model/dto/Haptic.ts';
import {Box, ButtonBase, Typography} from '@mui/material';
import type {JobDiscover} from '@/common/model/JobDiscover.ts';
import {JOB_TAB, type JobTab} from '@/page/common/model/JobModel';
import {MonitorPage, type MonitorFocus} from './monitor/MonitorPage';
import {usePlatformMobile} from '@/common/context/usePlatformMobile';
import {useSystemUtilService} from '@/common/context/useAppHooks.tsx';
import {usePagePanelNav} from '@/page/common/hooks/usePagePanelNav.ts';

export function JobPage(props: {active: boolean}) {
    const {t} = useI18n();
    const {active} = props;
    const mobileViewport = usePlatformMobile();
    const systemUtilService = useSystemUtilService();
    const [monitorEpoch, setMonitorEpoch] = useState(0);
    const [tab, setTab] = useState<JobTab>(JOB_TAB.OVERVIEW);
    const [monitorFocus, setMonitorFocus] = useState<MonitorFocus | null>(null);
    const [focusDiscoverId, setFocusDiscoverId] = useState<string | null>(null);
    const {resolvePanelProps, onPanelAnimationEnd} = usePagePanelNav(tab, mobileViewport);

    const triggerSelectionHaptic = (): void => {
        if (!mobileViewport) {
            return;
        }
        void systemUtilService.haptic({kind: HAPTIC_KIND.SELECTION});
    };

    const handleTabChange = (next: JobTab): void => {
        if (tab === next) {
            return;
        }
        triggerSelectionHaptic();
        setTab(next);
    };

    const handleDiscoverStarted = (discover: JobDiscover): void => {
        setTab(JOB_TAB.DISCOVER);
        setFocusDiscoverId(discover.id);
    };

    const handleOpenIndicator = (jobDeviceId: string, jobIndicatorId: string): void => {
        setMonitorFocus(prev => ({epoch: (prev?.epoch ?? 0) + 1, jobDeviceId, jobIndicatorId}));
        setTab(JOB_TAB.MONITOR);
    };

    const handleAccepted = (): void => {
        setMonitorEpoch(prev => prev + 1);
    };

    useEffect(() => {
        if (!active) {
            return;
        }
        setMonitorEpoch(prev => prev + 1);
    }, [active]);

    return (
        <Box className="tm-home-page tm-job-home">
            <Box className="tm-job-shell">
                <Box className="tm-job-shell-main">
                    <Box className="tm-job-shell-body">
                        <Box className="tm-job-tabs">
                            <ButtonBase disableRipple className={tab === JOB_TAB.OVERVIEW ? 'tm-job-tab is-active' : 'tm-job-tab'} onClick={() => handleTabChange(JOB_TAB.OVERVIEW)}>
                                <Typography component="span">{t('home.job.tabOverview')}</Typography>
                            </ButtonBase>
                            <ButtonBase disableRipple className={tab === JOB_TAB.MONITOR ? 'tm-job-tab is-active' : 'tm-job-tab'} onClick={() => handleTabChange(JOB_TAB.MONITOR)}>
                                <Typography component="span">{t('home.job.tabMonitor')}</Typography>
                            </ButtonBase>
                            <ButtonBase disableRipple className={tab === JOB_TAB.DISCOVER ? 'tm-job-tab is-active' : 'tm-job-tab'} onClick={() => handleTabChange(JOB_TAB.DISCOVER)}>
                                <Typography component="span">{t('home.job.tabDiscover')}</Typography>
                            </ButtonBase>
                        </Box>
                        <Box className="tm-job-main">
                            <Box {...resolvePanelProps(JOB_TAB.OVERVIEW)} onAnimationEnd={(event) => onPanelAnimationEnd(JOB_TAB.OVERVIEW, event)}>
                                <OverviewPage active={active && tab === JOB_TAB.OVERVIEW} onOpenMonitor={() => handleTabChange(JOB_TAB.MONITOR)} onOpenIndicator={handleOpenIndicator}/>
                            </Box>
                            <Box {...resolvePanelProps(JOB_TAB.MONITOR)} onAnimationEnd={(event) => onPanelAnimationEnd(JOB_TAB.MONITOR, event)}>
                                <MonitorPage listEpoch={monitorEpoch} focus={monitorFocus} onDiscoverStarted={handleDiscoverStarted}/>
                            </Box>
                            <Box {...resolvePanelProps(JOB_TAB.DISCOVER)} onAnimationEnd={(event) => onPanelAnimationEnd(JOB_TAB.DISCOVER, event)}>
                                <DiscoverPage active={active && tab === JOB_TAB.DISCOVER} focusId={focusDiscoverId} onFocusConsumed={() => setFocusDiscoverId(null)} onAccepted={handleAccepted}/>
                            </Box>
                        </Box>
                    </Box>
                </Box>
            </Box>
        </Box>
    );
}
