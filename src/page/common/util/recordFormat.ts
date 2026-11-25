import {translate} from '@/common/i18n/i18n.ts';
import type {SessionRecord} from '@/common/model/SessionRecord.ts';

export function formatRecordTime(millis: number): string {
    const date = new Date(millis);
    const pad = (value: number): string => String(value).padStart(2, '0');
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

export function formatRecordDuration(record: SessionRecord): string {
    if (record.ended_at <= record.started_at) {
        return translate('home.record.durationUnknown');
    }
    return formatSeconds((record.ended_at - record.started_at) / 1000);
}

export function formatSeconds(totalSeconds: number): string {
    const seconds = Math.max(0, Math.floor(totalSeconds));
    const minutes = Math.floor(seconds / 60);
    const rest = seconds % 60;
    if (minutes < 60) {
        return `${String(minutes).padStart(2, '0')}:${String(rest).padStart(2, '0')}`;
    }
    const hours = Math.floor(minutes / 60);
    return `${hours}:${String(minutes % 60).padStart(2, '0')}:${String(rest).padStart(2, '0')}`;
}

export function formatRecordSize(sizeBytes: number): string {
    if (sizeBytes < 1024) {
        return `${sizeBytes} B`;
    }
    if (sizeBytes < 1024 * 1024) {
        return `${(sizeBytes / 1024).toFixed(1)} KB`;
    }
    return `${(sizeBytes / 1024 / 1024).toFixed(1)} MB`;
}
