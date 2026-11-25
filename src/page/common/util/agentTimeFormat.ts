import {isNil} from '@/common/util/StringUtil.ts';
import {formatElapsed} from '@/page/common/hooks/useElapsed.ts';
import {translate, translateFormat} from '@/common/i18n/i18n.ts';

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

const MONTH_ABBR = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

function pad2(value: number): string {
    return String(value).padStart(2, '0');
}

export function isSameDay(left: number, right: number): boolean {
    const a = new Date(left);
    const b = new Date(right);
    return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
}

export function formatClock(millis: number): string {
    const date = new Date(millis);
    return `${pad2(date.getHours())}:${pad2(date.getMinutes())}`;
}

export function formatFullDateTime(millis: number): string {
    const date = new Date(millis);
    return `${date.getFullYear()}/${pad2(date.getMonth() + 1)}/${pad2(date.getDate())} ${pad2(date.getHours())}:${pad2(date.getMinutes())}:${pad2(date.getSeconds())}`;
}

function formatMonthDay(date: Date, withYear: boolean): string {
    const params = {y: date.getFullYear(), m: date.getMonth() + 1, mon: MONTH_ABBR[date.getMonth()], d: date.getDate()};
    return translateFormat(withYear ? 'agent.time.yearMonthDay' : 'agent.time.monthDay', params);
}

export function formatDayLabel(millis: number, now: number = Date.now()): string {
    if (isSameDay(millis, now)) {
        return translate('agent.time.today');
    }
    if (isSameDay(millis, now - DAY)) {
        return translate('agent.time.yesterday');
    }
    const date = new Date(millis);
    return formatMonthDay(date, date.getFullYear() !== new Date(now).getFullYear());
}

export function formatMessageTime(millis: number, now: number = Date.now()): string {
    const clock = formatClock(millis);
    if (isSameDay(millis, now)) {
        return clock;
    }
    return `${formatDayLabel(millis, now)} ${clock}`;
}

export function formatAgo(millis: number | null | undefined, now: number = Date.now()): string {
    if (isNil(millis) || millis <= 0) {
        return '';
    }
    const diff = Math.max(0, now - millis);
    if (diff < MINUTE) {
        return translate('agent.time.justNow');
    }
    if (diff < HOUR) {
        return translateFormat('agent.time.minutes', {n: Math.floor(diff / MINUTE)});
    }
    if (diff < DAY) {
        return translateFormat('agent.time.hours', {n: Math.floor(diff / HOUR)});
    }
    if (diff < 30 * DAY) {
        return translateFormat('agent.time.days', {n: Math.floor(diff / DAY)});
    }
    if (diff < 365 * DAY) {
        return translateFormat('agent.time.months', {n: Math.floor(diff / (30 * DAY))});
    }
    return translateFormat('agent.time.years', {n: Math.floor(diff / (365 * DAY))});
}

export function formatDuration(millis: number): string {
    return formatElapsed(millis);
}
