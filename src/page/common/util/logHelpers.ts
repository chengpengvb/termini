import {LOG_LEVEL} from '@/page/common/dto/terminal/LogEntry.ts';

export const LOG_LEVEL_TEXT: Record<number, string> = {
    [LOG_LEVEL.INFO]: 'INFO',
    [LOG_LEVEL.ERROR]: 'ERROR',
};

export const LOG_LEVEL_CLASS: Record<number, string> = {
    [LOG_LEVEL.INFO]: 'is-info',
    [LOG_LEVEL.ERROR]: 'is-error',
};

export function formatLogTime(timestamp: number): string {
    const date = new Date(timestamp);
    const hour = String(date.getHours()).padStart(2, '0');
    const minute = String(date.getMinutes()).padStart(2, '0');
    const second = String(date.getSeconds()).padStart(2, '0');
    return `${hour}:${minute}:${second}`;
}

