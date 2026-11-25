import {Grid} from './Grid';
import {List} from './List';
import {Empty} from './Empty';
import {Box} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {VIEW_MODE} from '@/common/const/Const.ts';
import {isEmpty} from '@/common/util/StringUtil.ts';
import {useCallback, useEffect, useState} from "react";
import {usePaneContext} from "@/common/components/tab";
import type {PortForward} from '@/common/model/PortForward.ts';
import {resolveErrorText} from '@/common/model/resp/ResultModel.ts';
import {usePortForwardService, useTabManagerService} from "@/common/context/useAppHooks.tsx";

export function Content(props: { active: boolean; viewMode: number; searchText: string | null; onBlankClick: () => void; switchVisible: () => void; setPortForward: (value: PortForward) => void; setDialogOpen: (open: boolean) => void }) {
    const {t, tf} = useI18n();
    const {paneId} = usePaneContext();
    const tabManagerService = useTabManagerService();
    const portForwardService = usePortForwardService();
    const [loaded, setLoaded] = useState<boolean>(false);
    const [error, setError] = useState<string | null>(null);
    const [forwards, setForwards] = useState<PortForward[]>([]);
    const {active, viewMode, searchText, onBlankClick, switchVisible, setPortForward, setDialogOpen} = props;

    const initForwards = useCallback((): void => {
        portForwardService.reloadAll().then(() => {
            setLoaded(true);
        }).catch(err => {
            setLoaded(true);
            setError(resolveErrorText(err));
        });
    }, [portForwardService]);

    useEffect(() => {
        return tabManagerService.addPaneOpenListener(pane => {
            if (paneId !== pane.id) {
                return;
            }
            initForwards();
        });
    }, [initForwards, paneId, tabManagerService]);

    useEffect(() => {
        if (!active) {
            return;
        }
        initForwards();
    }, [active, initForwards]);

    useEffect(() => {
        const offList = portForwardService.addForwardListChangeListener(list => {
            setForwards(list);
        });
        return () => {
            offList();
        };
    }, [portForwardService]);

    const resolveVisibleForwards = (): PortForward[] => {
        if (isEmpty(searchText)) {
            return forwards;
        }
        const keyword = searchText.toLowerCase();
        return forwards.filter(item => {
            if (isEmpty(item.name)) {
                return false;
            }
            return item.name.toLowerCase().includes(keyword);
        });
    };

    const visibleForwards = resolveVisibleForwards();

    return (
        <Box className="tm-home-scroll">
            {!loaded && forwards.length === 0 ? (
                <Box className="tm-home-loading-text">{t('home.common.loading')}</Box>
            ) : null}
            {loaded && error ? (
                <Box className="tm-home-error-text">{tf('home.common.loadFailed', {error})}</Box>
            ) : null}
            {loaded && !error && forwards.length === 0 ? (
                <Empty switchVisible={switchVisible}/>
            ) : null}
            {!error && forwards.length > 0 && viewMode === VIEW_MODE.GRID ? (
                <Grid forwards={visibleForwards} setPortForward={setPortForward} onBlankClick={onBlankClick} setDialogOpen={setDialogOpen}/>
            ) : null}
            {!error && forwards.length > 0 && viewMode === VIEW_MODE.LIST ? (
                <List forwards={visibleForwards} setPortForward={setPortForward} onBlankClick={onBlankClick} setDialogOpen={setDialogOpen}/>
            ) : null}
        </Box>
    );
}
