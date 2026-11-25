import {useCallback, useEffect, useRef, useState} from 'react';
import type {RespProvider} from '@/common/model/resp/RespProvider.ts';
import {useAccountService, useAiProvideService} from '@/common/context/useAppHooks.tsx';
import {AI_PROVIDE_KIND, AI_PROVIDE_SOURCE, type AiProvide} from '@/common/model/AiProvide.ts';

export interface AiProvideOptions {
    cloudList: AiProvide[];
    customList: AiProvide[];
    templateList: RespProvider[];
    refreshing: boolean;
    refresh: () => void;
}

export function useAiProvideOptions(): AiProvideOptions {
    const accountService = useAccountService();
    const aiProvideService = useAiProvideService();
    const requestRef = useRef(0);
    const [refreshing, setRefreshing] = useState<boolean>(true);
    const [cloudList, setCloudList] = useState<AiProvide[]>([]);
    const [customList, setCustomList] = useState<AiProvide[]>([]);
    const [templateList, setTemplateList] = useState<RespProvider[]>([]);

    const splitList = useCallback((list: AiProvide[]): void => {
        const visible = list.filter(item => item.hidden !== true);
        setCloudList(visible.filter(item => item.source === AI_PROVIDE_SOURCE.CLOUD && item.kind === AI_PROVIDE_KIND.CHAT));
        setCustomList(visible.filter(item => item.source === AI_PROVIDE_SOURCE.CUSTOM));
    }, []);

    const refresh = useCallback((): void => {
        const request = ++requestRef.current;
        setRefreshing(true);
        void aiProvideService.refresh().then((list) => {
            if (request === requestRef.current) {
                splitList(list);
            }
        }).catch(() => {
            // Cloud refresh can fail while signed out or offline. Keep the last local snapshot.
        }).finally(() => {
            if (request === requestRef.current) {
                setRefreshing(false);
            }
        });
    }, [aiProvideService, splitList]);

    useEffect(() => {
        let active = true;
        void aiProvideService.listTemplateProvider().then((list) => {
            if (active) {
                setTemplateList(list);
            }
        }).catch(() => {
            // Provider metadata is optional; model ids remain usable without it.
        });
        const offList = aiProvideService.addAiProvideListChangeListener((list) => {
            if (!active) {
                return;
            }
            requestRef.current += 1;
            splitList(list);
            setRefreshing(false);
        });
        const offAccount = accountService.addAccountChangeListener((account) => {
            if (account !== null) {
                refresh();
            }
        });
        const request = ++requestRef.current;
        setRefreshing(true);
        void aiProvideService.selectAll().then((list) => {
            if (active && request === requestRef.current) {
                splitList(list);
            }
        }).catch(() => {
            // A refresh below still gets a chance to populate the list.
        }).finally(() => {
            if (active && request === requestRef.current) {
                refresh();
            }
        });
        return () => {
            active = false;
            requestRef.current += 1;
            offList();
            offAccount();
        };
    }, [accountService, aiProvideService, refresh, splitList]);

    return {cloudList, customList, templateList, refreshing, refresh};
}
