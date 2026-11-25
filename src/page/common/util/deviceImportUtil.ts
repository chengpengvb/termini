import type {DeviceImportItem} from '@/common/model/DeviceImport.ts';

export function itemKey(item: DeviceImportItem): string {
    return `${item.host.trim().toLowerCase()}:${item.port}`;
}

export function matchItem(item: DeviceImportItem, keyword: string): boolean {
    if (item.name.toLowerCase().includes(keyword)) {
        return true;
    }
    if (item.host.toLowerCase().includes(keyword)) {
        return true;
    }
    return (item.username ?? '').toLowerCase().includes(keyword);
}
