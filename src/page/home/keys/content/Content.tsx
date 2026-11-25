import {Grid} from './Grid';
import {List} from './List';
import {Empty} from './Empty';
import {Box} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {VIEW_MODE} from "@/common/const/Const.ts";
import {isEmpty} from '@/common/util/StringUtil.ts';
import {useCallback, useEffect, useState} from "react";
import {usePaneContext} from "@/common/components/tab";
import type {ConfigKeyPair} from '@/common/model/ConfigKeyPair.ts';
import {resolveErrorText} from '@/common/model/resp/ResultModel.ts';
import {useConfigKeyPairService, useTabManagerService} from "@/common/context/useAppHooks.tsx";

export function Content(props: { active: boolean; viewMode: number; searchText: string | null; onBlankClick: () => void; switchVisible: () => void; setConfigKeyPair: (value: ConfigKeyPair) => void; setDialogOpen: (open: boolean) => void }) {
    const {t, tf} = useI18n();
    const {paneId} = usePaneContext();
    const tabManagerService = useTabManagerService();
    const configKeyPairService = useConfigKeyPairService();
    const [loaded, setLoaded] = useState<boolean>(false);
    const [error, setError] = useState<string | null>(null);
    const [keyPairs, setKeyPairs] = useState<ConfigKeyPair[]>([]);
    const {active, viewMode, searchText, onBlankClick, switchVisible, setConfigKeyPair, setDialogOpen} = props;

    const initKeyPairs = useCallback((): void => {
        configKeyPairService.reloadAll().then(() => {
            setLoaded(true);
        }).catch(err => {
            setLoaded(true);
            setError(resolveErrorText(err));
        });
    }, [configKeyPairService]);

    useEffect(() => {
        return tabManagerService.addPaneOpenListener(pane => {
            if (paneId !== pane.id) {
                return;
            }
            initKeyPairs();
        });
    }, [initKeyPairs, paneId, tabManagerService]);

    useEffect(() => {
        if (!active) {
            return;
        }
        initKeyPairs();
    }, [active, initKeyPairs]);

    useEffect(() => {
        const offList = configKeyPairService.addKeyPairListChangeListener(list => {
            setKeyPairs(list);
        });
        return () => {
            offList();
        };
    }, [configKeyPairService]);

    const resolveVisibleKeyPairs = (): ConfigKeyPair[] => {
        if (isEmpty(searchText)) {
            return keyPairs;
        }
        const keyword = searchText.toLowerCase();
        return keyPairs.filter(item => {
            if (isEmpty(item.name)) {
                return false;
            }
            return item.name.toLowerCase().includes(keyword);
        });
    };

    const visibleKeyPairs = resolveVisibleKeyPairs();

    return (
        <Box className="tm-home-scroll">
            {!loaded && keyPairs.length === 0 ? (
                <Box className="tm-home-loading-text">{t('home.common.loading')}</Box>
            ) : null}
            {loaded && error ? (
                <Box className="tm-home-error-text">{tf('home.common.loadFailed', {error})}</Box>
            ) : null}
            {loaded && !error && keyPairs.length === 0 ? (
                <Empty switchVisible={switchVisible}/>
            ) : null}
            {!error && keyPairs.length > 0 && viewMode === VIEW_MODE.GRID ? (
                <Grid keyPairs={visibleKeyPairs} setConfigKeyPair={setConfigKeyPair} onBlankClick={onBlankClick} setDialogOpen={setDialogOpen}/>
            ) : null}
            {!error && keyPairs.length > 0 && viewMode === VIEW_MODE.LIST ? (
                <List keyPairs={visibleKeyPairs} setConfigKeyPair={setConfigKeyPair} onBlankClick={onBlankClick} setDialogOpen={setDialogOpen}/>
            ) : null}
        </Box>
    );
}
