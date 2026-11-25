import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNotEmpty} from '@/common/util/StringUtil.ts';
import {Sparkline} from '../common/chart/Sparkline.tsx';
import {useEffect, useMemo, useRef, useState} from 'react';
import type {JobDevice} from '@/common/model/JobDevice.ts';
import type {JobDiscover} from '@/common/model/JobDiscover.ts';
import type {ChartSample} from '@/page/common/util/chartUtil.ts';
import {JOB_DEVICE_DRAWER_TAB} from '@/page/common/model/JobModel';
import {JOB_ALERT_STATUS} from '@/common/model/JobIndicatorAlert.ts';
import type {JobDevice2Indicator} from '@/common/model/JobDevice.ts';
import {Box, ButtonBase, IconButton, Typography} from '@mui/material';
import type {RespMetricItem} from '@/common/model/resp/RespMetricItem.ts';
import type {JobIndicatorAlert} from '@/common/model/JobIndicatorAlert.ts';
import {discoverStatusLabel, JOB_DISCOVER_STATUS} from '@/common/model/JobDiscover.ts';
import {Close, PushPin, PushPinOutlined, WarningAmberOutlined} from '@mui/icons-material';
import {JOB_ALERT_RULE_OP, ruleOpLabel, type JobIndicator} from '@/common/model/JobIndicator.ts';
import {useJobDeviceService, useJobDiscoverService, useJobIndicatorAlertService} from '@/common/context/useAppHooks.tsx';
import {resolveDeviceOsIcon, resolveDeviceOsIconChipClassName, resolveDeviceOsIconChipStyle} from '@/page/common/util/deviceUtil.ts';
import {displayThreshold, displayValue, formatAgo, latestMetricValue, resolveMetricStatus, toneOf, toSamples, type JobStatus} from '@/page/common/util/jobUtil.ts';

const TOP_ANOMALY_LIMIT = 20;
const WATERMARK_LIMIT = 10;
const LIVE_SAMPLE_LIMIT = 12;
const DEVICE_WALL_LIMIT = 24;
const RECENT_DISCOVER_LIMIT = 8;
const RECENT_DIAGNOSIS_LIMIT = 8;
const RECENT_ALERT_FETCH_LIMIT = 50;

const WALL_STATUS_ORDER: Record<OverviewStatus, number> = {firing: 0, error: 1, pending: 2, ok: 3, paused: 4};

type OverviewStatus = JobStatus | 'paused';

interface AnomalyRow {
    device: JobDevice;
    status: JobStatus;
    metric: RespMetricItem;
    link: JobDevice2Indicator;
}

interface WatermarkRow {
    ratio: number;
    firing: boolean;
    valueText: string;
    device: JobDevice;
    thresholdText: string;
    indicator: JobIndicator;
    link: JobDevice2Indicator;
}

interface SampleRow {
    device: JobDevice;
    status: JobStatus;
    valueText: string;
    updatedAt: number;
    link: JobDevice2Indicator;
    samples: ChartSample[];
}

interface OverviewPageProps {
    active: boolean;
    onOpenMonitor?: () => void;
    onOpenIndicator?: (jobDeviceId: string, jobIndicatorId: string) => void;
}

function metricKey(jobDeviceId: string, jobIndicatorId: string): string {
    return `${jobDeviceId}/${jobIndicatorId}`;
}

function metricStamp(metric: RespMetricItem): number {
    if (metric.updated_at == null) {
        return 0;
    }
    return metric.updated_at;
}

function anomalyStamp(row: AnomalyRow): number {
    if (row.metric.alert?.alert_at != null) {
        return row.metric.alert.alert_at;
    }
    return metricStamp(row.metric);
}

function resolveDeviceName(jobDevice: JobDevice): string {
    if (isNotEmpty(jobDevice.device?.name)) {
        return jobDevice.device.name;
    }
    if (isNotEmpty(jobDevice.device?.host)) {
        return jobDevice.device.host;
    }
    return jobDevice.device_id;
}

function waterFillClass(row: WatermarkRow): string {
    if (row.firing || row.ratio >= 1) {
        return 'tm-job-overview-water-fill is-firing';
    }
    if (row.ratio >= 0.7) {
        return 'tm-job-overview-water-fill is-warn';
    }
    return 'tm-job-overview-water-fill is-ok';
}

function watermarkRatio(op: number, value: number, threshold: number): number | null {
    if (!Number.isFinite(value) || !Number.isFinite(threshold) || threshold <= 0) {
        return null;
    }
    if (op === JOB_ALERT_RULE_OP.GT || op === JOB_ALERT_RULE_OP.GTE) {
        return value / threshold;
    }
    if (op === JOB_ALERT_RULE_OP.LT || op === JOB_ALERT_RULE_OP.LTE) {
        if (value <= 0) {
            return 1;
        }
        return threshold / value;
    }
    return null;
}

export function OverviewPage(props: OverviewPageProps) {
    const {t, tf} = useI18n();
    const jobDeviceService = useJobDeviceService();
    const jobDiscoverService = useJobDiscoverService();
    const {active, onOpenMonitor, onOpenIndicator} = props;
    const [collectGap, setCollectGap] = useState<number>(0);
    const flashStampRef = useRef<Map<string, number>>(new Map());
    const [jobDevices, setJobDevices] = useState<JobDevice[]>([]);
    const [discovers, setDiscovers] = useState<JobDiscover[]>([]);
    const jobIndicatorAlertService = useJobIndicatorAlertService();
    const [flashKeys, setFlashKeys] = useState<Set<string>>(new Set());
    const [recentAlerts, setRecentAlerts] = useState<JobIndicatorAlert[]>([]);
    const [metrics, setMetrics] = useState<Map<string, RespMetricItem>>(new Map());
    const [livePins, setLivePins] = useState<string[]>(() => jobDeviceService.getLivePins());

    const resolveDeviceStatus = (device: JobDevice): OverviewStatus => {
        if (!device.enabled) {
            return 'paused';
        }
        const links = device.indicators ?? [];
        let hasError = false;
        let hasOk = false;
        for (const link of links) {
            const status = resolveMetricStatus(metrics.get(metricKey(device.id, link.job_indicator_id)));
            if (status === 'firing') {
                return 'firing';
            }
            if (status === 'error') {
                hasError = true;
            } else if (status === 'ok') {
                hasOk = true;
            }
        }
        if (hasError) {
            return 'error';
        }
        if (!hasOk) {
            return 'pending';
        }
        return 'ok';
    };

    const statusByDevice = useMemo(() => {
        const map = new Map<string, OverviewStatus>();
        for (const device of jobDevices) {
            map.set(device.id, resolveDeviceStatus(device));
        }
        return map;
    }, [jobDevices, metrics]);

    const firingCountByDevice = useMemo(() => {
        const map = new Map<string, number>();
        for (const device of jobDevices) {
            let count = 0;
            for (const link of device.indicators ?? []) {
                if (resolveMetricStatus(metrics.get(metricKey(device.id, link.job_indicator_id))) === 'firing') {
                    count += 1;
                }
            }
            map.set(device.id, count);
        }
        return map;
    }, [jobDevices, metrics]);

    const lastCollectByDevice = useMemo(() => {
        const map = new Map<string, number>();
        for (const device of jobDevices) {
            let latest = 0;
            for (const link of device.indicators ?? []) {
                const metric = metrics.get(metricKey(device.id, link.job_indicator_id));
                if (metric != null) {
                    latest = Math.max(latest, metricStamp(metric));
                }
            }
            map.set(device.id, latest);
        }
        return map;
    }, [jobDevices, metrics]);

    const wallDevices = useMemo(() => {
        const list = [...jobDevices];
        list.sort((left, right) => {
            const order = WALL_STATUS_ORDER[statusByDevice.get(left.id) ?? 'pending'] - WALL_STATUS_ORDER[statusByDevice.get(right.id) ?? 'pending'];
            if (order !== 0) {
                return order;
            }
            return resolveDeviceName(left).localeCompare(resolveDeviceName(right));
        });
        return list.slice(0, DEVICE_WALL_LIMIT);
    }, [jobDevices, statusByDevice]);

    const counts = useMemo(() => {
        const next = {ok: 0, firing: 0, error: 0, pending: 0, paused: 0};
        for (const status of statusByDevice.values()) {
            next[status] += 1;
        }
        return next;
    }, [statusByDevice]);

    const anomalies = useMemo(() => {
        const rows: AnomalyRow[] = [];
        for (const device of jobDevices) {
            if (!device.enabled) {
                continue;
            }
            for (const link of device.indicators ?? []) {
                const metric = metrics.get(metricKey(device.id, link.job_indicator_id));
                if (metric == null) {
                    continue;
                }
                const status = resolveMetricStatus(metric);
                if (status !== 'firing' && status !== 'error') {
                    continue;
                }
                rows.push({device, link, metric, status});
            }
        }
        rows.sort((left, right) => {
            if (left.status !== right.status) {
                return left.status === 'firing' ? -1 : 1;
            }
            return anomalyStamp(right) - anomalyStamp(left);
        });
        return rows.slice(0, TOP_ANOMALY_LIMIT);
    }, [jobDevices, metrics]);

    const statItems = [
        {key: 'ok', count: counts.ok, label: t('home.job.overview.statOk')},
        {key: 'firing', count: counts.firing, label: t('home.job.overview.statFiring')},
        {key: 'error', count: counts.error, label: t('home.job.overview.statError')},
        {key: 'pending', count: counts.pending, label: t('home.job.overview.statPending')},
        {key: 'paused', count: counts.paused, label: t('home.job.overview.statPaused')},
    ];

    const resolveDotClass = (status: OverviewStatus): string => {
        if (status === 'paused') {
            return 'is-pending';
        }
        return `is-${status}`;
    };

    const resolveTileMeta = (device: JobDevice, status: OverviewStatus): string => {
        const parts: string[] = [];
        if (isNotEmpty(device.device?.host)) {
            parts.push(device.device.host);
        }
        if (status === 'paused') {
            parts.push(t('home.job.overview.statPaused'));
        } else {
            const firingCount = firingCountByDevice.get(device.id) ?? 0;
            if (firingCount > 0) {
                parts.push(tf('home.job.overview.tileAlertCount', {n: firingCount}));
            } else {
                parts.push(tf('home.job.overview.tileMetricCount', {n: (device.indicators ?? []).length}));
            }
            const lastCollect = lastCollectByDevice.get(device.id) ?? 0;
            if (lastCollect > 0) {
                parts.push(formatAgo(lastCollect));
            }
        }
        return parts.join(' · ');
    };

    const handleOpenDiagnosis = (alert: JobIndicatorAlert): void => {
        jobDeviceService.setDrawerTab(JOB_DEVICE_DRAWER_TAB.ALERT);
        onOpenIndicator?.(alert.job_device_id, alert.job_indicator_id);
    };

    const handleOpenMetric = (device: JobDevice, link: JobDevice2Indicator): void => {
        jobDeviceService.setDrawerTab(JOB_DEVICE_DRAWER_TAB.DETAIL);
        onOpenIndicator?.(device.id, link.job_indicator_id);
    };

    const handleOpenDevice = (device: JobDevice): void => {
        const links = device.indicators ?? [];
        const firing = links.find(link => resolveMetricStatus(metrics.get(metricKey(device.id, link.job_indicator_id))) === 'firing');
        const target = firing ?? links[0];
        if (target == null) {
            onOpenMonitor?.();
            return;
        }
        handleOpenMetric(device, target);
    };

    const handleToggleLivePin = (row: SampleRow): void => {
        setLivePins(jobDeviceService.toggleLivePin(metricKey(row.device.id, row.link.job_indicator_id)));
    };

    const handleFlashEnd = (key: string): void => {
        setFlashKeys(current => {
            if (!current.has(key)) {
                return current;
            }
            const next = new Set(current);
            next.delete(key);
            return next;
        });
    };

    const resolveFlashClass = (base: string, key: string): string => {
        if (flashKeys.has(key)) {
            return `${base} is-flash`;
        }
        return base;
    };

    const resolveAnomalyValue = (row: AnomalyRow): string => {
        if (row.status === 'error') {
            return t('home.job.displayCollectFailed');
        }
        const last = row.metric.values.length ? row.metric.values[row.metric.values.length - 1] : null;
        return displayValue(row.link.job_indicator, last);
    };

    const watermarks = useMemo(() => {
        const rows: WatermarkRow[] = [];
        for (const device of jobDevices) {
            if (!device.enabled) {
                continue;
            }
            for (const link of device.indicators ?? []) {
                const indicator = link.job_indicator;
                if (indicator == null || !isNotEmpty(indicator.alert_threshold)) {
                    continue;
                }
                const metric = metrics.get(metricKey(device.id, link.job_indicator_id));
                if (metric == null) {
                    continue;
                }
                const last = latestMetricValue(metric.values);
                if (last == null || !last.success || !isNotEmpty(last.value)) {
                    continue;
                }
                const ratio = watermarkRatio(indicator.alert_rule_op, Number(last.value), Number(indicator.alert_threshold));
                if (ratio == null) {
                    continue;
                }
                rows.push({
                    device,
                    link,
                    indicator,
                    ratio,
                    firing: resolveMetricStatus(metric) === 'firing',
                    valueText: displayValue(indicator, last),
                    thresholdText: displayThreshold(indicator, indicator.alert_threshold),
                });
            }
        }
        rows.sort((left, right) => right.ratio - left.ratio);
        return rows.slice(0, WATERMARK_LIMIT);
    }, [jobDevices, metrics]);

    const liveSamples = useMemo(() => {
        const rows: SampleRow[] = [];
        for (const device of jobDevices) {
            for (const link of device.indicators ?? []) {
                const metric = metrics.get(metricKey(device.id, link.job_indicator_id));
                if (metric == null) {
                    continue;
                }
                const last = latestMetricValue(metric.values);
                if (last == null || !last.success) {
                    continue;
                }
                rows.push({
                    link,
                    device,
                    updatedAt: metricStamp(metric),
                    samples: toSamples(metric.values),
                    status: resolveMetricStatus(metric),
                    valueText: displayValue(link.job_indicator, last),
                });
            }
        }
        const pinned: SampleRow[] = [];
        for (const key of livePins) {
            const row = rows.find(item => metricKey(item.device.id, item.link.job_indicator_id) === key);
            if (row != null) {
                pinned.push(row);
            }
        }
        const rest = rows.filter(row => !livePins.includes(metricKey(row.device.id, row.link.job_indicator_id)));
        rest.sort((left, right) => right.updatedAt - left.updatedAt);
        return [...pinned, ...rest].slice(0, LIVE_SAMPLE_LIMIT);
    }, [jobDevices, metrics, livePins]);

    const deviceNameById = useMemo(() => {
        const map = new Map<string, string>();
        for (const device of jobDevices) {
            map.set(device.id, resolveDeviceName(device));
        }
        return map;
    }, [jobDevices]);

    const recentDiscovers = useMemo(() => {
        const list = [...discovers];
        list.sort((left, right) => right.timestamp - left.timestamp);
        return list.slice(0, RECENT_DISCOVER_LIMIT);
    }, [discovers]);

    const diagnoses = useMemo(() => {
        return recentAlerts
            .filter(alert => isNotEmpty(alert.ai_summary))
            .slice(0, RECENT_DIAGNOSIS_LIMIT);
    }, [recentAlerts]);

    const resolveDiscoverStatusClass = (status: number): string => {
        if (status === JOB_DISCOVER_STATUS.FAILED) {
            return 'is-firing';
        }
        if (status === JOB_DISCOVER_STATUS.SUCCESS) {
            return 'is-ok';
        }
        return 'is-pending';
    };

    const collectGapText = (): string => {
        const days = Math.floor(collectGap / (24 * 3600_000));
        if (days >= 1) {
            return tf('home.job.overview.collectGapDays', {n: days});
        }
        return tf('home.job.overview.collectGapHours', {n: Math.max(1, Math.round(collectGap / 3600_000))});
    };

    const handleCollectGapClose = (): void => {
        setCollectGap(0);
    };

    useEffect(() => {
        if (!active) {
            return;
        }
        void jobDeviceService.selectAllDetail().then(setJobDevices);
    }, [jobDeviceService, active]);

    useEffect(() => {
        if (!active) {
            return;
        }
        void jobDeviceService.takeCollectGap().then(setCollectGap);
    }, [jobDeviceService, active]);

    useEffect(() => {
        void jobDiscoverService.selectAll().then(setDiscovers);
        return jobDiscoverService.addJobDiscoverListChangeListener(setDiscovers);
    }, [jobDiscoverService]);

    useEffect(() => {
        if (!active) {
            return;
        }
        void jobIndicatorAlertService.selectRecent(RECENT_ALERT_FETCH_LIMIT).then(setRecentAlerts);
        return jobIndicatorAlertService.addAlertListChangeListener(() => {
            void jobIndicatorAlertService.selectRecent(RECENT_ALERT_FETCH_LIMIT).then(setRecentAlerts);
        });
    }, [jobIndicatorAlertService, active]);

    useEffect(() => {
        const changed = new Set<string>();
        metrics.forEach((metric, key) => {
            const previous = flashStampRef.current.get(key);
            const stamp = metricStamp(metric);
            flashStampRef.current.set(key, stamp);
            if (previous != null && previous !== stamp) {
                changed.add(key);
            }
        });
        if (changed.size === 0) {
            return;
        }
        setFlashKeys(current => new Set([...current, ...changed]));
    }, [metrics]);

    useEffect(() => {
        const offs: (() => void)[] = [];
        for (const device of jobDevices) {
            offs.push(jobDeviceService.addMetricListener(device.id, metric => {
                setMetrics(current => new Map(current).set(metricKey(metric.job_device_id, metric.job_indicator_id), metric));
            }));
            const indicatorIds = (device.indicators ?? []).map(link => link.job_indicator_id);
            if (indicatorIds.length === 0) {
                continue;
            }
            void jobDeviceService.selectMetric(device.id, indicatorIds).then(list => {
                setMetrics(current => {
                    const next = new Map(current);
                    for (const item of list) {
                        const key = metricKey(item.job_device_id, item.job_indicator_id);
                        const existing = next.get(key);
                        if (existing != null && metricStamp(existing) > metricStamp(item)) {
                            continue;
                        }
                        next.set(key, item);
                    }
                    return next;
                });
            });
        }
        return () => {
            offs.forEach(off => off());
        };
    }, [jobDeviceService, jobDevices]);

    return (
        <Box className="tm-job-overview">
            {collectGap > 0 ? (
                <Box className="tm-job-overview-banner">
                    <WarningAmberOutlined className="tm-job-overview-banner-icon"/>
                    <Box className="tm-job-overview-banner-main">
                        <Typography className="tm-job-overview-banner-title">{collectGapText()}</Typography>
                        <Typography className="tm-job-overview-banner-desc">{t('home.job.overview.collectGapHint')}</Typography>
                    </Box>
                    <IconButton size="small" className="tm-job-overview-banner-close" onClick={handleCollectGapClose} aria-label={t('common.close')}>
                        <Close fontSize="small"/>
                    </IconButton>
                </Box>
            ) : null}
            <Box className="tm-job-overview-stats">
                {statItems.map(item => (
                    <Box key={item.key} className={`tm-job-overview-stat is-${item.key}`}>
                        <Typography className="tm-job-overview-stat-count">{item.count}</Typography>
                        <Typography className="tm-job-overview-stat-label">{item.label}</Typography>
                    </Box>
                ))}
            </Box>
            <Box className="tm-job-overview-body">
                <Box className="tm-job-overview-panel is-wall">
                    <Box className="tm-job-overview-panel-head">
                        <Typography className="tm-job-overview-panel-title">{t('home.job.overview.deviceWall')}</Typography>
                        <Typography className="tm-job-overview-panel-count">{jobDevices.length}</Typography>
                    </Box>
                    {jobDevices.length === 0 ? (
                        <Typography className="tm-job-overview-empty">{t('home.job.overview.devicesEmpty')}</Typography>
                    ) : (
                        <Box className="tm-job-overview-wall">
                            {wallDevices.map(device => {
                                const status = statusByDevice.get(device.id) ?? 'pending';
                                const osType = device.device?.os_type ?? 0;
                                return (
                                    <ButtonBase key={device.id} disableRipple className={`tm-job-overview-tile is-${status}`} onClick={() => handleOpenDevice(device)}>
                                        <Box className="tm-job-overview-tile-head">
                                            <Box className={resolveDeviceOsIconChipClassName(osType)} style={resolveDeviceOsIconChipStyle(osType)}>
                                                {resolveDeviceOsIcon(osType, 'tm-device-item-os-icon')}
                                            </Box>
                                            <Typography className="tm-job-overview-tile-name">{resolveDeviceName(device)}</Typography>
                                            <Box className={`tm-job-status-dot tm-job-overview-tile-dot ${resolveDotClass(status)}`}/>
                                        </Box>
                                        <Typography className={`tm-job-overview-tile-meta ${(firingCountByDevice.get(device.id) ?? 0) > 0 ? 'is-firing' : ''}`}>
                                            {resolveTileMeta(device, status)}
                                        </Typography>
                                    </ButtonBase>
                                );
                            })}
                            {jobDevices.length > wallDevices.length ? (
                                <ButtonBase disableRipple className="tm-job-overview-tile is-more" onClick={onOpenMonitor}>
                                    <Typography className="tm-job-overview-tile-more">{tf('home.job.overview.wallMore', {n: jobDevices.length - wallDevices.length})}</Typography>
                                </ButtonBase>
                            ) : null}
                        </Box>
                    )}
                </Box>
                <Box className="tm-job-overview-panel is-water">
                    <Box className="tm-job-overview-panel-head">
                        <Typography className="tm-job-overview-panel-title">{t('home.job.overview.waterTitle')}</Typography>
                        <Typography className="tm-job-overview-panel-count">{watermarks.length}</Typography>
                    </Box>
                    {watermarks.length === 0 ? (
                        <Typography className="tm-job-overview-empty">{t('home.job.overview.waterEmpty')}</Typography>
                    ) : (
                        <Box className="tm-job-overview-list">
                            {watermarks.map(row => (
                                <ButtonBase key={`${row.device.id}/${row.link.id}`} disableRipple className={resolveFlashClass('tm-job-overview-water-row', metricKey(row.device.id, row.link.job_indicator_id))} onClick={() => handleOpenMetric(row.device, row.link)} onAnimationEnd={() => handleFlashEnd(metricKey(row.device.id, row.link.job_indicator_id))}>
                                    <Box className="tm-job-overview-water-head">
                                        <Typography className="tm-job-overview-feed-title">{row.indicator.name}</Typography>
                                        <Typography className="tm-job-overview-feed-meta">{resolveDeviceName(row.device)}</Typography>
                                        <Typography className={`tm-job-overview-water-value ${row.firing ? 'is-firing' : ''}`}>
                                            {row.valueText} / {ruleOpLabel(row.indicator.alert_rule_op)} {row.thresholdText}
                                        </Typography>
                                    </Box>
                                    <Box className="tm-job-overview-water-track">
                                        <Box className={waterFillClass(row)} style={{width: `${Math.min(row.ratio, 1) * 100}%`}}/>
                                    </Box>
                                </ButtonBase>
                            ))}
                        </Box>
                    )}
                </Box>
                <Box className="tm-job-overview-panel is-top">
                    <Box className="tm-job-overview-panel-head">
                        <Typography className="tm-job-overview-panel-title">{t('home.job.overview.topTitle')}</Typography>
                        <Typography className="tm-job-overview-panel-count">{anomalies.length}</Typography>
                    </Box>
                    {anomalies.length === 0 ? (
                        <Typography className="tm-job-overview-empty">{t('home.job.overview.topEmpty')}</Typography>
                    ) : (
                        <Box className="tm-job-overview-list">
                            {anomalies.map(row => (
                                <ButtonBase key={`${row.device.id}/${row.link.id}`} disableRipple className="tm-job-overview-top-row" onClick={() => handleOpenMetric(row.device, row.link)}>
                                    <Box className={`tm-job-status-dot is-${row.status}`}/>
                                    <Box className="tm-job-overview-top-main">
                                        <Typography className="tm-job-overview-top-indicator">{row.link.job_indicator?.name ?? row.link.job_indicator_id}</Typography>
                                        <Typography className="tm-job-overview-top-device">{resolveDeviceName(row.device)} · {formatAgo(anomalyStamp(row))}</Typography>
                                    </Box>
                                    <Typography className={`tm-job-overview-top-value is-${row.status}`}>{resolveAnomalyValue(row)}</Typography>
                                    <Sparkline samples={toSamples(row.metric.values)} tone={toneOf(row.status)}/>
                                </ButtonBase>
                            ))}
                        </Box>
                    )}
                </Box>
                <Box className="tm-job-overview-panel is-live">
                    <Box className="tm-job-overview-panel-head">
                        <Typography className="tm-job-overview-panel-title">{t('home.job.overview.liveTitle')}</Typography>
                        <Typography className="tm-job-overview-panel-count">{liveSamples.length}</Typography>
                    </Box>
                    {liveSamples.length === 0 ? (
                        <Typography className="tm-job-overview-empty">{t('home.job.overview.liveEmpty')}</Typography>
                    ) : (
                        <Box className="tm-job-overview-list">
                            {liveSamples.map(row => {
                                const key = metricKey(row.device.id, row.link.job_indicator_id);
                                const pinned = livePins.includes(key);
                                return (
                                    <ButtonBase key={`${row.device.id}/${row.link.id}`} disableRipple className={resolveFlashClass('tm-job-overview-top-row', key)} onClick={() => handleOpenMetric(row.device, row.link)} onAnimationEnd={() => handleFlashEnd(key)}>
                                        <Box className={`tm-job-status-dot is-${row.status}`}/>
                                        <Box className="tm-job-overview-top-main">
                                            <Typography className="tm-job-overview-top-indicator">{row.link.job_indicator?.name ?? row.link.job_indicator_id}</Typography>
                                            <Typography className="tm-job-overview-top-device">{resolveDeviceName(row.device)} · {formatAgo(row.updatedAt)}</Typography>
                                        </Box>
                                        <Typography className="tm-job-overview-live-value">{row.valueText}</Typography>
                                        <Sparkline samples={row.samples} tone={toneOf(row.status)}/>
                                        <Box
                                            component="span"
                                            title={pinned ? t('home.job.overview.unpin') : t('home.job.overview.pin')}
                                            className={pinned ? 'tm-job-overview-pin is-pinned' : 'tm-job-overview-pin'}
                                            onClick={event => {
                                                event.stopPropagation();
                                                handleToggleLivePin(row);
                                            }}
                                        >
                                            {pinned ? <PushPin/> : <PushPinOutlined/>}
                                        </Box>
                                    </ButtonBase>
                                );
                            })}
                        </Box>
                    )}
                </Box>
                <Box className="tm-job-overview-panel is-discover">
                    <Box className="tm-job-overview-panel-head">
                        <Typography className="tm-job-overview-panel-title">{t('home.job.overview.recentDiscover')}</Typography>
                        <Typography className="tm-job-overview-panel-count">{recentDiscovers.length}</Typography>
                    </Box>
                    {recentDiscovers.length === 0 ? (
                        <Typography className="tm-job-overview-empty">{t('home.job.overview.discoverEmpty')}</Typography>
                    ) : (
                        <Box className="tm-job-overview-list">
                            {recentDiscovers.map(discover => (
                                <Box key={discover.id} className="tm-job-overview-feed-row">
                                    <Box className="tm-job-overview-feed-head">
                                        <Typography className="tm-job-overview-feed-title">{deviceNameById.get(discover.job_device_id) ?? discover.job_device_id}</Typography>
                                        <Typography className={`tm-job-overview-feed-state ${resolveDiscoverStatusClass(discover.status)}`}>{discoverStatusLabel(discover.status)}</Typography>
                                        <Typography className="tm-job-overview-feed-ago">{formatAgo(discover.timestamp)}</Typography>
                                    </Box>
                                    {isNotEmpty(discover.summary) ? (
                                        <Typography className="tm-job-overview-feed-text">{discover.summary}</Typography>
                                    ) : null}
                                </Box>
                            ))}
                        </Box>
                    )}
                </Box>
                <Box className="tm-job-overview-panel is-diagnosis">
                    <Box className="tm-job-overview-panel-head">
                        <Typography className="tm-job-overview-panel-title">{t('home.job.overview.aiDiagnosis')}</Typography>
                        <Typography className="tm-job-overview-panel-count">{diagnoses.length}</Typography>
                    </Box>
                    {diagnoses.length === 0 ? (
                        <Typography className="tm-job-overview-empty">{t('home.job.overview.diagnosisEmpty')}</Typography>
                    ) : (
                        <Box className="tm-job-overview-list">
                            {diagnoses.map(alert => (
                                <ButtonBase key={alert.id} disableRipple className="tm-job-overview-feed-row" onClick={() => handleOpenDiagnosis(alert)}>
                                    <Box className="tm-job-overview-feed-head">
                                        <Box className={`tm-job-status-dot ${alert.status === JOB_ALERT_STATUS.FIRING ? 'is-firing' : 'is-ok'}`}/>
                                        <Typography className="tm-job-overview-feed-title">{alert.job_indicator?.name ?? alert.job_indicator_id}</Typography>
                                        <Typography className="tm-job-overview-feed-meta">{deviceNameById.get(alert.job_device_id) ?? alert.job_device_id}</Typography>
                                        <Typography className="tm-job-overview-feed-ago">{formatAgo(alert.alert_at)}</Typography>
                                    </Box>
                                    <Typography className="tm-job-overview-feed-text">{alert.ai_summary}</Typography>
                                </ButtonBase>
                            ))}
                        </Box>
                    )}
                </Box>
            </Box>
        </Box>
    );
}
