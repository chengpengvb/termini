import {translateFormat} from '@/common/i18n/i18n.ts';
import type {DeviceService} from '@/service/DeviceService';

export function resolveDeviceListDetail(deviceService: DeviceService, deviceIds: string[] | undefined): string | null {
    if (!deviceIds || deviceIds.length === 0) {
        return null;
    }
    const labels: string[] = [];
    for (const deviceId of deviceIds) {
        if (!deviceId) {
            continue;
        }
        labels.push(deviceService.resolveDeviceLabel(deviceId) ?? deviceId);
    }
    if (labels.length === 0) {
        return null;
    }
    return labels.length === 1 ? labels[0] : `${translateFormat('common.agent.batchDevices', {count: labels.length})}: ${labels.join(', ')}`;
}
