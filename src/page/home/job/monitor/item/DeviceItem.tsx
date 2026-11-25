import {useEffect, useState} from 'react';
import {DeviceLabel} from './DeviceLabel.tsx';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {IndicatorItem} from './IndicatorItem.tsx';
import {resolveMetricStatus} from '@/page/common/util/jobUtil.ts';
import {useJobDeviceService} from '@/common/context/useAppHooks.tsx';
import type {RespMetricItem} from '@/common/model/resp/RespMetricItem.ts';
import type {JobDevice, JobDevice2Indicator} from '@/common/model/JobDevice.ts';
import {JOB_MONITOR_VIEW, type JobMonitorView} from '@/page/common/model/JobModel';
import {Add, KeyboardArrowDown, KeyboardArrowRight, TravelExplore} from '@mui/icons-material';
import {Box, ButtonBase, CircularProgress, Collapse, Switch, Typography} from '@mui/material';

export interface DeviceItemProps {
    view: JobMonitorView;
    jobDevice: JobDevice;
    discovering?: boolean;
    addSelected?: boolean;
    selectedLinkId?: string | null;
    onDiscover?: (jobDevice: JobDevice) => void;
    onAddIndicator?: (jobDevice: JobDevice) => void;
    onEnabledChange?: (id: string, enabled: boolean) => void;
    onSelectIndicator?: (jobDevice: JobDevice, link: JobDevice2Indicator) => void;
}

function metricStamp(metric: RespMetricItem): number {
    if (metric.updated_at == null) {
        return 0;
    }
    return metric.updated_at;
}

function mergeMetricList(current: Map<string, RespMetricItem>, list: RespMetricItem[]): Map<string, RespMetricItem> {
    const next = new Map<string, RespMetricItem>();
    for (const item of list) {
        const existing = current.get(item.job_indicator_id);
        if (existing != null && metricStamp(existing) > metricStamp(item)) {
            next.set(item.job_indicator_id, existing);
            continue;
        }
        next.set(item.job_indicator_id, item);
    }
    return next;
}

export function DeviceItem(props: DeviceItemProps) {
    const {t} = useI18n();
    const [saving, setSaving] = useState(false);
    const jobDeviceService = useJobDeviceService();
    const indicators = props.jobDevice.indicators ?? [];
    const indicatorIds = indicators.map(link => link.job_indicator_id);
    const indicatorIdsKey = indicatorIds.join(',');
    const [metrics, setMetrics] = useState<Map<string, RespMetricItem>>(new Map());
    const [open, setOpen] = useState(() => jobDeviceService.getExpanded(props.jobDevice.id));
    const firing = indicators.some(link => resolveMetricStatus(metrics.get(link.job_indicator_id)) === 'firing');
    const {view, jobDevice, discovering, selectedLinkId, addSelected, onDiscover, onAddIndicator, onEnabledChange, onSelectIndicator} = props;

    const handleExplore = (): void => {
        if (discovering) {
            return;
        }
        onDiscover?.(jobDevice);
    };

    const handleAdd = (): void => {
        onAddIndicator?.(jobDevice);
    };

    const handleEnabledChange = (enabled: boolean): void => {
        if (saving || enabled === jobDevice.enabled) {
            return;
        }
        setSaving(true);
        onEnabledChange?.(jobDevice.id, enabled);
        void jobDeviceService.setEnabled(jobDevice.id, enabled).then(() => {
            setSaving(false);
        }).catch(() => {
            onEnabledChange?.(jobDevice.id, !enabled);
            setSaving(false);
        });
    };

    useEffect(() => {
        let cancelled = false;
        if (indicatorIds.length === 0) {
            setMetrics(new Map());
        } else {
            void jobDeviceService.selectMetric(jobDevice.id, indicatorIds).then(list => {
                if (cancelled) {
                    return;
                }
                setMetrics(current => mergeMetricList(current, list));
            });
        }
        const off = jobDeviceService.addMetricListener(jobDevice.id, metric => {
            setMetrics(current => new Map(current).set(metric.job_indicator_id, metric));
        });
        return () => {
            cancelled = true;
            off();
        };
    }, [jobDeviceService, jobDevice.id, indicatorIdsKey]);

    return (
        <Box className={jobDevice.enabled ? 'tm-job-device-card' : 'tm-job-device-card is-disabled'}>
            <Box className="tm-job-device-head">
                <ButtonBase disableRipple className="tm-job-device-head-toggle" aria-expanded={open} onClick={() => {
                        const next = !open;
                        setOpen(next);
                        jobDeviceService.setExpanded(jobDevice.id, next);
                    }}>
                    <Box component="span" className="tm-job-device-chevron">
                        {open ? <KeyboardArrowDown sx={{fontSize: 16}}/> : <KeyboardArrowRight sx={{fontSize: 16}}/>}
                    </Box>
                    <DeviceLabel jobDevice={jobDevice} firing={firing}/>
                </ButtonBase>
                <Box className={jobDevice.enabled ? 'tm-job-device-head-actions' : 'tm-job-device-head-actions is-paused'} onClick={event => event.stopPropagation()}>
                    <ButtonBase disableRipple className={discovering ? 'tm-job-device-explore is-running' : 'tm-job-device-explore'} disabled={discovering} onClick={handleExplore} aria-label={t('home.job.exploreMetrics')} title={discovering ? t('home.job.exploring') : t('home.job.exploreMetrics')}>
                        {discovering ? <CircularProgress size={13} thickness={5} className="tm-job-device-explore-spin"/> : <TravelExplore sx={{fontSize: 16}}/>}
                    </ButtonBase>
                    <Switch size="small" className="tm-job-device-switch" checked={jobDevice.enabled} disabled={saving} onChange={event => handleEnabledChange(event.target.checked)}/>
                </Box>
            </Box>
            <Collapse in={open} timeout={0}>
                <Box className={view === JOB_MONITOR_VIEW.GRID ? 'tm-job-metric-list is-grid' : 'tm-job-metric-list'}>
                    {indicators.map(link => (
                        <IndicatorItem key={link.id} link={link} metric={metrics.get(link.job_indicator_id)} selected={selectedLinkId === link.id} onSelect={link => onSelectIndicator?.(jobDevice, link)}/>
                    ))}
                    <ButtonBase disableRipple className={addSelected ? 'tm-device-item tm-job-metric-row is-add is-selected' : 'tm-device-item tm-job-metric-row is-add'} onClick={handleAdd} aria-label={t('home.job.addMetric')}>
                        <Add sx={{fontSize: 16}}/>
                        <Typography className="tm-job-metric-name">{t('home.job.addMetric')}</Typography>
                    </ButtonBase>
                </Box>
            </Collapse>
        </Box>
    );
}
