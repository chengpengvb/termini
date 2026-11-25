import {useEffect} from 'react';
import type {ReactNode} from 'react';
import type {Device} from '@/common/model/Device.ts';
import {usePaneContext} from '@/common/components/tab';
import type {ReqDevice} from '@/common/model/req/ReqDevice.ts';
import {resolveDeviceTabIcon} from '@/page/common/util/deviceUtil.ts';
import {useDeviceService, useTabManagerService} from '@/common/context/useAppHooks.tsx';

export function DeviceTabBarSync(props: { deviceId: string, protocol?: number, actionIcon?: ReactNode, actionTitle?: string, onActionClick?: () => void }) {
    const {deviceId, protocol, actionIcon, actionTitle, onActionClick} = props;
    const {paneId} = usePaneContext();
    const deviceService = useDeviceService();
    const tabManagerService = useTabManagerService();

    useEffect(() => {
        const resolveDeviceFromList = (list: ReqDevice[]): Device | null => {
            for (const item of list) {
                if (item.device.id === deviceId) {
                    return item.device;
                }
            }
            return null;
        };

        const applyDeviceTabBar = (device: Device): void => {
            const pane = tabManagerService.getPane(paneId);
            if (!pane) {
                return;
            }
            tabManagerService.updatePaneBar(paneId, {
                actionIcon,
                actionTitle,
                onActionClick,
                title: device.name,
                canClose: pane.tabBar.canClose,
                canMerge: pane.tabBar.canMerge,
                titleClassName: pane.tabBar.titleClassName,
                icon: resolveDeviceTabIcon(device.os_type, protocol),
            });
        };

        const syncFromList = (list: ReqDevice[]): void => {
            const device = resolveDeviceFromList(list);
            if (!device) {
                tabManagerService.closePane(paneId);
                return;
            }
            applyDeviceTabBar(device);
        };

        const cached = deviceService.getCachedDevice(deviceId);
        if (cached) {
            applyDeviceTabBar(cached);
        }
        return deviceService.addDeviceListChangeListener(syncFromList);
    }, [deviceId, protocol, paneId, deviceService, tabManagerService, actionIcon, actionTitle, onActionClick]);

    return null;
}
