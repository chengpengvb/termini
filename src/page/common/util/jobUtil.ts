import {isEmpty, isNil} from '@/common/util/StringUtil.ts';
import {formatDisplayText} from '@/page/common/util/unitUtil.ts';
import {translate, translateFormat} from '@/common/i18n/i18n.ts';
import type {ChartSample} from '@/page/common/util/chartUtil.ts';
import {resolveDeviceOsLabel} from '@/page/common/util/deviceUtil.ts';
import {JOB_ALERT_FIRE_TYPE} from '@/common/model/JobIndicatorAlert.ts';
import type {JobIndicatorValue} from '@/common/model/JobIndicatorValue.ts';
import type {JobDevice, JobDevice2Indicator} from '@/common/model/JobDevice.ts';
import {RESP_METRIC_STATUS, type RespMetricItem} from '@/common/model/resp/RespMetricItem.ts';
import {buildIndicatorArg, INDICATOR_ARG_TYPE, JOB_ALERT_AI_ON, JOB_ALERT_RULE_OP, JOB_DISPLAY_UNIT, JOB_PARSER_TYPE, JOB_PROTOCOL, type JobIndicator} from '@/common/model/JobIndicator.ts';

export const DEFAULT_REGEX_EXPR = '(?P<value>[\\d.]+)';
export const DEFAULT_SHELL_SCRIPT = "df -P / | awk 'NR==2 {print $5}' | tr -d '%'";
export const DEFAULT_SNMP_SPEC = '{"op":"get","oids":["1.3.6.1.2.1.1.3.0"]}';
export const DEFAULT_INTERVAL_SECS = 300;
const SSH_ARG_TEMPLATE = {id: 'ssh-script', protocol: JOB_PROTOCOL.SSH, argType: INDICATOR_ARG_TYPE.SHELL, labelI18n: 'job.indicator.arg.ssh.script'};

export type JobTone = 'ok' | 'firing' | 'neutral';
export type JobStatus = 'ok' | 'firing' | 'error' | 'pending';

export function filterJobDevice(jobDevice: JobDevice, keyword: string): JobDevice | null {
    const device = jobDevice.device;
    if (device != null) {
        if (device.name.toLowerCase().includes(keyword)) {
            return jobDevice;
        }
        if (device.host.toLowerCase().includes(keyword)) {
            return jobDevice;
        }
        if (resolveDeviceOsLabel(device.os_type).toLowerCase().includes(keyword)) {
            return jobDevice;
        }
    }
    const indicators = jobDevice.indicators;
    if (indicators == null) {
        return null;
    }
    const matched = indicators.filter(link => link.job_indicator != null && link.job_indicator.name.toLowerCase().includes(keyword));
    if (matched.length === 0) {
        return null;
    }
    return {...jobDevice, indicators: matched};
}

export function isDraftLink(link: JobDevice2Indicator | null | undefined): boolean {
    if (isNil(link) || isNil(link.job_indicator)) {
        return false;
    }
    return isEmpty(link.job_indicator.id);
}

export function createDraftLink(jobDevice: JobDevice): JobDevice2Indicator {
    return {
        id: '',
        interval_secs: DEFAULT_INTERVAL_SECS,
        job_device_id: jobDevice.id,
        job_indicator_id: '',
        job_indicator: {
            id: '',
            name: '',
            sort: 0,
            enabled: true,
            protocol: JOB_PROTOCOL.SSH,
            parse_expr: DEFAULT_REGEX_EXPR,
            alert_ai_on: JOB_ALERT_AI_ON.OFF,
            is_default: false,
            parser_type: JOB_PARSER_TYPE.REGEX,
            display_unit: JOB_DISPLAY_UNIT.NONE,
            alert_rule_op: JOB_ALERT_RULE_OP.GT,
            alert_enabled: false,
            transform_expr: '',
            alert_threshold: '',
            alert_sensitivity: 1,
            range_min: null,
            range_max: null,
            arg: buildIndicatorArg(SSH_ARG_TEMPLATE, DEFAULT_SHELL_SCRIPT, ''),
        },
    };
}

export function indicatorCommand(indicator: JobIndicator | null | undefined): string {
    if (isNil(indicator) || isNil(indicator.arg)) {
        return '';
    }
    return indicator.arg.value ?? '';
}

export function resolveMetricStatus(metric: RespMetricItem | null | undefined): JobStatus {
    if (isNil(metric) || metric.status === RESP_METRIC_STATUS.PENDING) {
        return 'pending';
    }
    if (metric.status === RESP_METRIC_STATUS.ERROR) {
        return 'error';
    }
    if (metric.alert) {
        return 'firing';
    }
    return 'ok';
}

export function statusLabel(status: JobStatus): string {
    return translate(`home.job.status.${status}`);
}

export function toneOf(status: JobStatus): JobTone {
    if (status === 'ok') {
        return 'ok';
    }
    if (status === 'firing') {
        return 'firing';
    }
    return 'neutral';
}

export function formatInterval(secs: number): string {
    if (secs >= 3600 && secs % 3600 === 0) {
        return `${secs / 3600}h`;
    }
    if (secs >= 60) {
        return `${Math.round(secs / 60)}m`;
    }
    return `${secs}s`;
}

export function formatTime(millis: number | null | undefined): string {
    if (isNil(millis) || millis <= 0) {
        return '—';
    }
    const date = new Date(millis);
    const month = String(date.getMonth() + 1).padStart(2, '0');
    const day = String(date.getDate()).padStart(2, '0');
    const hours = String(date.getHours()).padStart(2, '0');
    const minutes = String(date.getMinutes()).padStart(2, '0');
    const seconds = String(date.getSeconds()).padStart(2, '0');
    return `${month}-${day} ${hours}:${minutes}:${seconds}`;
}

export function formatAgo(millis: number | null | undefined, emptyLabel?: string): string {
    const empty = emptyLabel ?? translate('home.job.ago.notCollected');
    if (isNil(millis) || millis <= 0) {
        return empty;
    }
    const diff = Math.max(0, Date.now() - millis);
    if (diff < 60_000) {
        return translate('home.job.ago.justNow');
    }
    if (diff < 3600_000) {
        return translateFormat('home.job.ago.minutes', {n: Math.floor(diff / 60_000)});
    }
    if (diff < 24 * 3600_000) {
        return translateFormat('home.job.ago.hours', {n: Math.floor(diff / 3600_000)});
    }
    if (diff < 30 * 24 * 3600_000) {
        return translateFormat('home.job.ago.days', {n: Math.floor(diff / (24 * 3600_000))});
    }
    if (diff < 365 * 24 * 3600_000) {
        return translateFormat('home.job.ago.months', {n: Math.floor(diff / (30 * 24 * 3600_000))});
    }
    return translateFormat('home.job.ago.years', {n: Math.floor(diff / (365 * 24 * 3600_000))});
}

export function indicatorUnit(indicator: JobIndicator | null | undefined): string {
    if (isNil(indicator) || isNil(indicator.arg)) {
        return '';
    }
    return indicator.arg.unit ?? '';
}

export function indicatorDisplayUnit(indicator: JobIndicator | null | undefined): number {
    return indicator?.display_unit ?? JOB_DISPLAY_UNIT.NONE;
}

export function latestMetricValue(values: JobIndicatorValue[] | null | undefined): JobIndicatorValue | null {
    if (isNil(values) || values.length === 0) {
        return null;
    }
    return values[values.length - 1] ?? null;
}

export function displayValue(indicator: JobIndicator | null | undefined, value: JobIndicatorValue | null | undefined): string {
    if (isNil(value)) {
        return '—';
    }
    if (!value.success) {
        return translate('home.job.displayCollectFailed');
    }
    if (isEmpty(value.value)) {
        return '—';
    }
    return formatDisplayText(value.value, indicatorDisplayUnit(indicator), indicatorUnit(indicator));
}

export function displayThreshold(indicator: JobIndicator | null | undefined, raw: string | null | undefined): string {
    if (isEmpty(raw)) {
        return '—';
    }
    return formatDisplayText(raw, indicatorDisplayUnit(indicator), indicatorUnit(indicator));
}

export function displayAlertTriggerValue(indicator: JobIndicator | null | undefined, alert: {fire_type?: number | null; alert_value: string}): string {
    if (alert.fire_type === JOB_ALERT_FIRE_TYPE.COLLECT_FAILED) {
        return '—';
    }
    return displayThreshold(indicator, alert.alert_value);
}

export function displayAlertResolvedValue(indicator: JobIndicator | null | undefined, alert: {resolved_type?: number | null; resolved_value?: string | null}): string {
    if (alert.resolved_type == null) {
        return '—';
    }
    if (alert.resolved_value == null || alert.resolved_value === '') {
        return '—';
    }
    return displayThreshold(indicator, alert.resolved_value);
}

export function toSamples(values: JobIndicatorValue[]): ChartSample[] {
    const samples: ChartSample[] = [];
    for (const value of values) {
        if (!value.success || isEmpty(value.value)) {
            continue;
        }
        const numeric = Number(value.value);
        if (!Number.isFinite(numeric)) {
            continue;
        }
        samples.push({t: value.updated_at, v: numeric});
    }
    return samples.sort((left, right) => left.t - right.t);
}
