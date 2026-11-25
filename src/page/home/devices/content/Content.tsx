import {Grid} from './Grid';
import {List} from './List';
import {Empty} from './Empty';
import {Box, Typography} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {VIEW_MODE} from "@/common/const/Const.ts";
import {isEmpty} from '@/common/util/StringUtil.ts';
import {useCallback, useEffect, useState} from "react";
import {usePaneContext} from "@/common/components/tab";
import type {ReqDevice} from '@/common/model/req/ReqDevice.ts';
import {resolveErrorText} from '@/common/model/resp/ResultModel.ts';
import {useDeviceService, useTabManagerService} from "@/common/context/useAppHooks.tsx";

export function Content(props: { active: boolean; viewMode: number; searchText: string | null; onBlankClick: () => void; switchVisible: () => void, setReqDevice: (reqDevice: ReqDevice) => void, setDrawerVisible: (visible: boolean) => void }) {
    const {t, tf} = useI18n();
    const {paneId} = usePaneContext();
    const deviceService = useDeviceService();
    const tabManagerService = useTabManagerService();
    const [loading, setLoading] = useState<boolean>(false);
    const [error, setError] = useState<string | null>(null);
    const [devices, setDevices] = useState<ReqDevice[]>([]);
    const {active, viewMode, searchText, onBlankClick, switchVisible, setReqDevice, setDrawerVisible} = props;

    const loadDevices = (): void => {
        setLoading(true)
        deviceService.selectAllWithConfig().then(() => {
            setLoading(false)
            setError(null)
        }).catch(err => {
            setLoading(false)
            setError(resolveErrorText(err));
        });
    };

    const initDevices = useCallback((): void => {
        loadDevices();
    }, [deviceService]);

    useEffect(() => {
        return tabManagerService.addPaneOpenListener(pane => {
            if (paneId !== pane.id) {
                return;
            }
            initDevices();
        });
    }, [initDevices, paneId, tabManagerService]);

    useEffect(() => {
        if (!active) {
            return;
        }
        initDevices();
    }, [active, initDevices]);

    useEffect(() => {
        const offDevices = deviceService.addDeviceListChangeListener(list => {
            setDevices(list);
        });
        return () => {
            offDevices();
        };
    }, [deviceService]);

    const resolveVisibleDevices = (): ReqDevice[] => {
        if (isEmpty(searchText)) {
            return devices;
        }
        const keyword = searchText.toLowerCase();
        return devices.filter(item => {
            if (item.device.name.toLowerCase().includes(keyword)) {
                return true;
            }
            return item.device.host.toLowerCase().includes(keyword);
        });
    };

    const visibleDevices = resolveVisibleDevices();
    const hasDevices = devices.length > 0;

    return (
        <Box className="tm-home-scroll">
            {loading && !hasDevices ? (
                <Typography className="tm-home-loading-text">{t('home.common.loading')}</Typography>
            ) : null}
            {error ? (
                <Typography className="tm-home-error-text">{tf('home.common.loadFailed', {error})}</Typography>
            ) : null}
            {!loading && !error && !hasDevices ? (
                <Empty switchVisible={switchVisible}/>
            ) : null}
            {hasDevices && viewMode === VIEW_MODE.GRID ? (
                <Grid devices={visibleDevices} setReqDevice={setReqDevice} onBlankClick={onBlankClick} setDrawerVisible={setDrawerVisible}/>
            ) : null}
            {hasDevices && viewMode === VIEW_MODE.LIST ? (
                <List devices={visibleDevices} setReqDevice={setReqDevice} onBlankClick={onBlankClick} setDrawerVisible={setDrawerVisible}/>
            ) : null}
        </Box>
    );
}
