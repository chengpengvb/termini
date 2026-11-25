import {Empty} from './Empty';
import {Box} from '@mui/material';
import {LocalShell} from './LocalShell';
import {HistoryList} from './HistoryList';
import {useEffect, useState} from 'react';
import type {ReqDevice} from '@/common/model/req/ReqDevice.ts';
import type {HistoryLogin} from '@/common/model/dto/HistoryLogin.ts';
import {useDeviceService, useHistoryLoginService} from '@/common/context/useAppHooks.tsx';

export function Content(props: {onEmptyClick: () => void}) {
    const deviceService = useDeviceService();
    const historyLoginService = useHistoryLoginService();
    const [deviceList, setDeviceList] = useState<ReqDevice[]>([]);
    const [historyList, setHistoryList] = useState<HistoryLogin[]>(() => historyLoginService.list());

    useEffect(() => {
        const off = deviceService.addDeviceListChangeListener((list) => {
            setDeviceList(list);
        });
        void deviceService.selectAllWithConfig();
        return off;
    }, [deviceService]);

    useEffect(() => {
        setHistoryList(historyLoginService.list());
        return historyLoginService.addListChangeListener((list) => {
            setHistoryList(list);
        });
    }, [historyLoginService]);

    const deviceMap = new Map<string, ReqDevice>();
    for (const item of deviceList) {
        if (!item.device.id) {
            continue;
        }
        deviceMap.set(item.device.id, item);
    }

    const visibleHistory = historyList.filter((item) => deviceMap.has(item.device_id));

    return (
        <Box className="tm-history-scroll">
            <Box className="tm-history-body">
                <LocalShell/>
                {visibleHistory.length === 0 ? (
                    <Empty onTitleClick={props.onEmptyClick}/>
                ) : (
                    <HistoryList items={visibleHistory} deviceMap={deviceMap}/>
                )}
            </Box>
        </Box>
    );
}
