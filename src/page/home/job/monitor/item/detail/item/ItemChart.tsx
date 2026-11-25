import type {MouseEvent} from 'react';
import {Refresh} from '@mui/icons-material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNil} from '@/common/util/StringUtil.ts';
import {useEffect, useRef, useState} from 'react';
import {Box, ButtonBase, Typography} from '@mui/material';
import {formatDisplay} from '@/page/common/util/unitUtil.ts';
import {JOB_DISPLAY_UNIT} from '@/common/model/JobIndicator.ts';
import {resolveErrorText} from '@/common/model/resp/ResultModel.ts';
import type {JobDevice2Indicator} from '@/common/model/JobDevice.ts';
import {formatTime, indicatorDisplayUnit, indicatorUnit, isDraftLink, toSamples} from '@/page/common/util/jobUtil.ts';
import {useJobDeviceService, useJobIndicatorService, useJobIndicatorValueService} from '@/common/context/useAppHooks.tsx';
import {buildHistoryGeometry, CHART_RANGES, CHART_SAMPLE_MIN, type ChartPadding, type ChartRangeKey, type ChartSample, formatChartTick, resolveChartRange, resolveHoverIndex, toChartNumber} from '@/page/common/util/chartUtil.ts';

export interface ItemChartProps {
    jobDeviceId: string;
    link: JobDevice2Indicator;
    onError: (message: string | null) => void;
}

const CHART_WIDTH = 520;
const CHART_HEIGHT = 180;
const CHART_PADDING: ChartPadding = {top: 12, left: 36, right: 12, bottom: 28};

function tipLeftPercent(x: number, width: number): number {
    return Math.min(86, Math.max(14, (x / Math.max(width, 1)) * 100));
}

export function ItemChart(props: ItemChartProps) {
    const {t, tf} = useI18n();
    const {jobDeviceId, link, onError} = props;
    const draft = isDraftLink(link);
    const indicator = link.job_indicator;
    const unit = indicatorUnit(indicator);
    const [running, setRunning] = useState(false);
    const jobDeviceService = useJobDeviceService();
    const plotRef = useRef<HTMLDivElement | null>(null);
    const jobIndicatorService = useJobIndicatorService();
    const displayUnit = indicatorDisplayUnit(indicator);
    const [samples, setSamples] = useState<ChartSample[]>([]);
    const [rangeKey, setRangeKey] = useState<ChartRangeKey>('1h');
    const jobIndicatorValueService = useJobIndicatorValueService();
    const [hoverIndex, setHoverIndex] = useState<number | null>(null);
    const [chartSize, setChartSize] = useState({width: CHART_WIDTH, height: CHART_HEIGHT});
    const range = resolveChartRange(rangeKey);
    const threshold = toChartNumber(indicator?.alert_threshold);
    const rangeMin = indicator?.range_min ?? Number.NaN;
    const rangeMax = indicator?.range_max ?? Number.NaN;
    const tone = samples.length > 0 ? 'ok' : 'neutral';
    const hasChart = samples.length >= CHART_SAMPLE_MIN;

    const formatAxis = (value: number): string => formatDisplay(value, displayUnit, '').replace(' ', '');

    const formatValue = (value: number): string => formatDisplay(value, displayUnit, unit);

    const handleHoverLeave = (): void => {
        setHoverIndex(null);
    };

    const handleHoverMove = (event: MouseEvent<HTMLDivElement>): void => {
        const rect = event.currentTarget.getBoundingClientRect();
        if (rect.width <= 0) {
            return;
        }
        setHoverIndex(resolveHoverIndex((event.clientX - rect.left) / rect.width, samples.length, chartSize.width, CHART_PADDING));
    };

    const handleRefresh = (): void => {
        if (running || draft) {
            return;
        }
        setRunning(true);
        onError(null);
        void jobIndicatorService.runOnce(link.id).then(collected => {
            const last = collected[collected.length - 1];
            if (last != null && !last.success && last.error != null) {
                onError(last.error);
            }
            return jobIndicatorValueService.selectByRange(jobDeviceId, link.job_indicator_id, range.durationMs);
        }).then(values => {
            setHoverIndex(null);
            setSamples(toSamples(values));
            setRunning(false);
        }).catch(err => {
            setRunning(false);
            onError(resolveErrorText(err));
        });
    };

    useEffect(() => {
        if (draft) {
            setHoverIndex(null);
            setSamples([]);
            return;
        }
        let cancelled = false;
        const load = (): void => {
            void jobIndicatorValueService.selectByRange(jobDeviceId, link.job_indicator_id, range.durationMs).then(values => {
                if (cancelled) {
                    return;
                }
                setHoverIndex(null);
                setSamples(toSamples(values));
                const last = values[values.length - 1];
                if (last != null && !last.success && last.error != null) {
                    onError(last.error);
                }
            }).catch(() => {
                if (cancelled) {
                    return;
                }
                setSamples([]);
            });
        };
        load();
        const off = jobDeviceService.addMetricListener(jobDeviceId, metric => {
            if (metric.job_indicator_id !== link.job_indicator_id) {
                return;
            }
            load();
        });
        return () => {
            cancelled = true;
            off();
        };
    }, [draft, jobDeviceService, jobIndicatorValueService, jobDeviceId, link.job_indicator_id, range.durationMs]);

    useEffect(() => {
        if (!hasChart) {
            return;
        }
        const el = plotRef.current;
        if (el == null) {
            return;
        }
        const apply = (width: number, height: number): void => {
            if (width <= 0 || height <= 0) {
                return;
            }
            const nextW = Math.round(width);
            const nextH = Math.round(height);
            setChartSize(prev => (prev.width === nextW && prev.height === nextH ? prev : {width: nextW, height: nextH}));
        };
        apply(el.clientWidth, el.clientHeight);
        const observer = new ResizeObserver(entries => {
            const entry = entries[0];
            if (entry == null) {
                return;
            }
            apply(entry.contentRect.width, entry.contentRect.height);
        });
        observer.observe(el);
        return () => observer.disconnect();
    }, [hasChart]);

    const refreshBtn = (
        <ButtonBase disableRipple className={running ? 'tm-job-icon-btn is-busy' : 'tm-job-icon-btn'} disabled={running || draft} onClick={handleRefresh} aria-label={t('home.job.collectNow')} title={draft ? t('home.job.collectNowDraft') : t('home.job.collectNow')}>
            <Refresh fontSize="small"/>
        </ButtonBase>
    );

    if (!hasChart) {
        return (
            <Box className={`tm-job-chart is-${tone}`}>
                <Box className="tm-job-chart-ranges">
                    {CHART_RANGES.map(item => (
                        <ButtonBase key={item.key} disableRipple className={item.key === rangeKey ? 'tm-job-chart-range is-active' : 'tm-job-chart-range'} onClick={() => setRangeKey(item.key)}>
                            {item.label}
                        </ButtonBase>
                    ))}
                </Box>
                <Typography className="tm-job-chart-empty">{t('home.job.chartEmpty')}</Typography>
                <Box className="tm-job-chart-foot">
                    <Typography className="tm-job-chart-caption">{tf('home.job.chartCaption', {range: range.label, n: 0})}</Typography>
                    {refreshBtn}
                </Box>
            </Box>
        );
    }

    const last = samples[samples.length - 1];
    const geometry = buildHistoryGeometry(samples, threshold, rangeMin, rangeMax, chartSize.width, chartSize.height, CHART_PADDING);
    const hoverPoint = isNil(hoverIndex) ? null : geometry.points[Math.min(hoverIndex, geometry.points.length - 1)];
    const hoverSample = isNil(hoverIndex) ? null : samples[Math.min(hoverIndex, samples.length - 1)];

    return (
        <Box className={`tm-job-chart is-${tone}`}>
            <Box className="tm-job-chart-ranges">
                {CHART_RANGES.map(item => (
                    <ButtonBase key={item.key} disableRipple className={item.key === rangeKey ? 'tm-job-chart-range is-active' : 'tm-job-chart-range'} onClick={() => setRangeKey(item.key)}>
                        {item.label}
                    </ButtonBase>
                ))}
            </Box>
            <Box ref={plotRef} className="tm-job-chart-plot" onMouseMove={handleHoverMove} onMouseLeave={handleHoverLeave}>
                <svg width="100%" height="100%" viewBox={`0 0 ${chartSize.width} ${chartSize.height}`} preserveAspectRatio="none" aria-label={t('home.job.chartAria')}>
                    {geometry.grids.map(grid => (
                        <g key={grid.y}>
                            <line className="tm-job-chart-grid" x1={geometry.startX} x2={geometry.endX} y1={grid.y} y2={grid.y}/>
                            <text className="tm-job-chart-axis" x={geometry.startX - 6} y={grid.y + 3} textAnchor="end">
                                {displayUnit === JOB_DISPLAY_UNIT.NONE ? grid.label : formatAxis(grid.value)}
                            </text>
                        </g>
                    ))}
                    {geometry.thresholdY != null ? (
                        <line className="tm-job-chart-threshold" x1={geometry.startX} x2={geometry.endX} y1={geometry.thresholdY} y2={geometry.thresholdY} strokeDasharray="4 3"/>
                    ) : null}
                    <polygon className="tm-job-chart-area" points={geometry.area}/>
                    <polyline className="tm-job-chart-line" fill="none" strokeWidth="2" strokeLinejoin="round" strokeLinecap="round" points={geometry.line}/>
                    <circle className="tm-job-chart-dot" cx={geometry.dotX} cy={geometry.dotY} r="3.5"/>
                    {hoverPoint != null ? (
                        <>
                            <line className="tm-job-chart-cross" x1={hoverPoint.x} x2={hoverPoint.x} y1={CHART_PADDING.top} y2={geometry.baseY}/>
                            <circle className="tm-job-chart-dot" cx={hoverPoint.x} cy={hoverPoint.y} r="3.5"/>
                        </>
                    ) : null}
                    <text className="tm-job-chart-axis" x={geometry.startX} y={chartSize.height - 8}>{formatChartTick(samples[0].t, range.durationMs)}</text>
                    <text className="tm-job-chart-axis" x={geometry.endX} y={chartSize.height - 8} textAnchor="end">{formatChartTick(last.t, range.durationMs)}</text>
                </svg>
                {hoverSample != null && hoverPoint != null ? (
                    <Box className="tm-job-chart-tip" style={{left: `${tipLeftPercent(hoverPoint.x, chartSize.width)}%`}}>
                        <Typography className="tm-job-chart-tip-time">{formatTime(hoverSample.t)}</Typography>
                        <Typography className="tm-job-chart-tip-value">{formatValue(hoverSample.v)}</Typography>
                    </Box>
                ) : null}
            </Box>
            <Box className="tm-job-chart-foot">
                <Typography className="tm-job-chart-caption">
                    {tf('home.job.chartCaptionFull', {range: range.label, n: samples.length, current: formatValue(last.v)})}
                    {Number.isFinite(threshold) ? ` · ${tf('home.common.thresholdValue', {value: formatValue(threshold)})}` : ''}
                </Typography>
                {refreshBtn}
            </Box>
        </Box>
    );
}
