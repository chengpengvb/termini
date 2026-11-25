import {Box, Button} from '@mui/material';
import {useEffect, useState} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {usePaneContext} from '@/common/components/tab';
import {useTabManagerService} from '@/common/context/useAppHooks.tsx';
import {AlertDialog} from '@/common/components/dialog/AlertDialog.tsx';

export interface BottomProps {
    selectedProtocols: number[];
    doConnect: (protocols: number[]) => void;
}

export function Bottom(props: BottomProps) {
    const {t} = useI18n();
    const {paneId} = usePaneContext();
    const {selectedProtocols, doConnect} = props;
    const tabManagerService = useTabManagerService();
    const canContinue = selectedProtocols.length > 0;
    const [alertInfo, setAlertInfo] = useState<{ showAlert: boolean, message: string | null }>({showAlert: false, message: null});

    const onClose = (): void => {
        tabManagerService.closePane(paneId);
    };

    const handleContinue = (): void => {
        if (!canContinue) {
            return;
        }
        doConnect(selectedProtocols);
    };

    const handleAlertClose = (): void => {
        setAlertInfo({showAlert: false, message: null});
    };

    const isTypingTarget = (target: EventTarget | null): boolean => {
        if (!(target instanceof HTMLElement)) {
            return false;
        }
        if (target.isContentEditable) {
            return true;
        }
        const tag = target.tagName;
        return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT';
    };

    const isThisPaneFront = (): boolean => {
        const activeTabId = tabManagerService.getActiveTabId();
        if (activeTabId == null) {
            return false;
        }
        const tab = tabManagerService.getTab(activeTabId);
        if (tab == null) {
            return false;
        }
        if (!tab.panes.some(item => item.id === paneId)) {
            return false;
        }
        return tab.focusedPaneId === paneId;
    };

    useEffect(() => {
        const onKeyDown = (event: KeyboardEvent): void => {
            if (event.key !== 'Enter' || !canContinue) {
                return;
            }
            if (isTypingTarget(event.target)) {
                return;
            }
            if (!isThisPaneFront()) {
                return;
            }
            event.preventDefault();
            handleContinue();
        };
        window.addEventListener('keydown', onKeyDown);
        return () => window.removeEventListener('keydown', onKeyDown);
    }, [canContinue, selectedProtocols, doConnect, paneId, tabManagerService]);

    return (
        <>
            <Box className="tm-protocol-select-actions">
                <Button disableRipple onClick={onClose} className="tm-btn tm-btn-secondary">{t('common.close')}</Button>
                <Button disableRipple disabled={!canContinue} onClick={handleContinue} className="tm-btn tm-btn-primary">
                    {t('common.continue')}
                </Button>
            </Box>
            <AlertDialog duration={1500} open={alertInfo.showAlert} handleClose={handleAlertClose} message={alertInfo.message} type="error"/>
        </>
    );
}
