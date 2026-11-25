import {Search} from '@mui/icons-material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {useEffect, useRef, useState} from 'react';
import {isEmpty} from '@/common/util/StringUtil.ts';
import {Item} from '@/page/home/history/content/Item.tsx';
import type {ReqDevice} from '@/common/model/req/ReqDevice.ts';
import {Box, Button, InputBase, Typography} from '@mui/material';
import {useDeviceService} from '@/common/context/useAppHooks.tsx';
import {usePlatformMobile} from '@/common/context/usePlatformMobile';
import {resolveDeviceTabIcon} from '@/page/common/util/deviceUtil.ts';
import type {KeyboardEvent, MouseEvent, ReactNode, RefObject} from 'react';

export interface HostSearchProps {
    placeholder: string;
    trailing?: ReactNode;
    barClassName: string;
    confirmLabel?: string;
    iconClassName: string;
    inputClassName: string;
    inputInnerClassName?: string;
    onOpenChange?: (open: boolean) => void;
    onQueryChange?: (query: string) => void;
    onConfirm: (reqDevice: ReqDevice) => void;
    inputRef?: RefObject<HTMLInputElement | null>;
}

export function HostSearch(props: HostSearchProps) {
    const {t} = useI18n();
    const [open, setOpen] = useState(false);
    const deviceService = useDeviceService();
    const mobileViewport = usePlatformMobile();
    const [searchText, setSearchText] = useState('');
    const keyword = searchText.trim().toLowerCase();
    const listRef = useRef<HTMLDivElement | null>(null);
    const rootRef = useRef<HTMLDivElement | null>(null);
    const innerRef = useRef<HTMLInputElement | null>(null);
    const [selectedIndex, setSelectedIndex] = useState(0);
    const [devices, setDevices] = useState<ReqDevice[]>([]);
    const {placeholder, barClassName, iconClassName, inputClassName, inputInnerClassName, trailing, confirmLabel, inputRef, onConfirm, onOpenChange, onQueryChange} = props;

    const filtered = devices.filter(item => {
        if (isEmpty(item.device.id)) {
            return false;
        }
        if (keyword.length === 0) {
            return true;
        }
        if (item.device.name.toLowerCase().includes(keyword)) {
            return true;
        }
        return item.device.host.toLowerCase().includes(keyword);
    });
    const selected = filtered[selectedIndex];
    const canConfirm = open && selected != null;

    const bindInputRef = (el: HTMLInputElement | null): void => {
        innerRef.current = el;
        if (inputRef != null) {
            inputRef.current = el;
        }
    };

    const closePanel = (): void => {
        setOpen(false);
        onOpenChange?.(false);
        innerRef.current?.blur();
    };

    const handleConfirm = (): void => {
        if (!open) {
            return;
        }
        if (selected == null) {
            return;
        }
        const root = rootRef.current;
        const active = document.activeElement;
        if (root == null || active == null || !root.contains(active)) {
            return;
        }
        const panel = root.closest('[data-tm-page-panel="true"]');
        if (panel != null && panel.getAttribute('data-tm-page-panel-active') !== 'true') {
            return;
        }
        onConfirm(selected);
        closePanel();
    };

    const moveSelected = (delta: number): void => {
        if (filtered.length === 0) {
            return;
        }
        setSelectedIndex(current => (current + delta + filtered.length) % filtered.length);
    };

    const handleFocus = (): void => {
        setOpen(true);
        onOpenChange?.(true);
    };

    const handleBlur = (): void => {
        setOpen(false);
        onOpenChange?.(false);
    };

    const handleChange = (value: string): void => {
        setSearchText(value);
        onQueryChange?.(value);
    };

    const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>): void => {
        if (event.key === 'Escape') {
            event.preventDefault();
            closePanel();
            return;
        }
        if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
            event.preventDefault();
            moveSelected(event.key === 'ArrowDown' ? 1 : -1);
            return;
        }
        if (event.key === 'Enter') {
            if (!open) {
                return;
            }
            event.preventDefault();
            event.stopPropagation();
            handleConfirm();
        }
    };

    const handleCandidateMouseDown = (event: MouseEvent<HTMLDivElement>): void => {
        event.preventDefault();
    };

    const handleConfirmMouseDown = (event: MouseEvent<HTMLButtonElement>): void => {
        event.preventDefault();
    };

    const resolveConnectClassName = (): string => {
        if (mobileViewport) {
            return 'tm-home-search-connect';
        }
        if (canConfirm) {
            return 'tm-btn tm-btn-compact tm-btn-contrast';
        }
        return 'tm-btn tm-btn-compact tm-btn-ghost';
    };

    const renderConfirm = () => {
        if (confirmLabel == null) {
            return null;
        }
        return (
            <Button type="button" disabled={!canConfirm} variant="text" disableRipple className={resolveConnectClassName()} onMouseDown={handleConfirmMouseDown} onPointerDown={handleConfirmMouseDown} onClick={handleConfirm}>
                {confirmLabel}
            </Button>
        );
    };

    useEffect(() => {
        const off = deviceService.addDeviceListChangeListener(list => {
            setDevices(list);
        });
        void deviceService.selectAllWithConfig();
        return off;
    }, [deviceService]);

    useEffect(() => {
        setSelectedIndex(0);
    }, [keyword]);

    useEffect(() => {
        if (!open) {
            return;
        }
        const list = listRef.current;
        if (list == null) {
            return;
        }
        const row = list.querySelector(`[data-index="${selectedIndex}"]`);
        if (!(row instanceof HTMLElement)) {
            return;
        }
        row.scrollIntoView({block: 'nearest'});
    }, [open, selectedIndex]);

    return (
        <Box ref={rootRef} className={mobileViewport ? 'tm-host-search is-mobile' : 'tm-host-search'}>
            <Box className="tm-host-search-row">
                <Box className={barClassName}>
                    <Search className={iconClassName}/>
                    <InputBase className={inputClassName} inputProps={inputInnerClassName == null ? undefined : {className: inputInnerClassName}} inputRef={bindInputRef} placeholder={placeholder} value={searchText} onChange={event => handleChange(event.target.value)} onFocus={handleFocus} onBlur={handleBlur} onKeyDown={handleKeyDown}/>
                    {trailing}
                    {mobileViewport ? null : renderConfirm()}
                </Box>
                {mobileViewport ? renderConfirm() : null}
            </Box>
            {open ? (
                <Box className="tm-history-search-candidate" onMouseDown={handleCandidateMouseDown} onPointerDown={handleCandidateMouseDown}>
                    <Typography className="tm-history-section-title">{t('home.common.remoteHosts')}</Typography>
                    <Box ref={listRef} className="tm-history-search-candidate-list">
                        {filtered.length === 0 ? (
                            <Box className="tm-home-loading-text">{t('common.noMatch')}</Box>
                        ) : filtered.map((item, index) => (
                            <Box key={item.device.id} data-index={index} onMouseEnter={() => setSelectedIndex(index)} onPointerDown={() => setSelectedIndex(index)}>
                                <Item title={item.device.name} meta={item.device.host} icon={resolveDeviceTabIcon(item.device.os_type)} selected={index === selectedIndex} onClick={() => {
                                    onConfirm(item);
                                    closePanel();
                                }}/>
                            </Box>
                        ))}
                    </Box>
                </Box>
            ) : null}
        </Box>
    );
}
