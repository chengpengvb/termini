import {Box} from '@mui/material';
import type {MouseEvent} from 'react';
import {useRef, useState} from 'react';
import {Content} from './content/Content';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {usePaneContext} from '@/common/components/tab';
import type {ReqDevice} from '@/common/model/req/ReqDevice.ts';
import {HostSearch} from '@/page/home/components/HostSearch.tsx';
import {resolveDeviceTabIcon} from '@/page/common/util/deviceUtil.ts';
import {useTabManagerService} from '@/common/context/useAppHooks.tsx';
import {SelectProtocol} from '@/page/protocol/common/select/SelectProtocol.tsx';

export function HistoryPage() {
    const {paneId} = usePaneContext();
    const tabManagerService = useTabManagerService();
    const inputRef = useRef<HTMLInputElement | null>(null);
    const [searchFocused, setSearchFocused] = useState(false);
    const {t} = useI18n();

    const handleConfirm = (reqDevice: ReqDevice): void => {
        tabManagerService.openTab({
            panes: [{
                tabBar: {
                    canClose: true,
                    title: reqDevice.device.name,
                    icon: resolveDeviceTabIcon(reqDevice.device.os_type),
                },
                content: <SelectProtocol reqDevice={reqDevice}/>,
            }],
        });
        tabManagerService.closePane(paneId);
    };

    const handleEmptyClick = (): void => {
        inputRef.current?.focus({preventScroll: true});
    };

    const handleMaskMouseDown = (event: MouseEvent<HTMLDivElement>): void => {
        event.preventDefault();
        setSearchFocused(false);
        inputRef.current?.blur();
    };

    return (
        <Box className={searchFocused ? 'tm-history-page is-search-open' : 'tm-history-page'}>
            {searchFocused ? <Box className="tm-history-search-mask" onMouseDown={handleMaskMouseDown} onPointerDown={handleMaskMouseDown} onClick={handleMaskMouseDown}/> : null}
            <Box className="tm-history-search-wrap">
                <HostSearch inputRef={inputRef} placeholder={t('home.history.searchPlaceholder')} barClassName="tm-history-search" iconClassName="tm-history-search-icon" inputClassName="tm-history-search-input" trailing={<Box className="tm-history-search-hint">Ctrl+K</Box>} onOpenChange={setSearchFocused} onConfirm={handleConfirm}/>
            </Box>
            <Content onEmptyClick={handleEmptyClick}/>
        </Box>
    );
}
