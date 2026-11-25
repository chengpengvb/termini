import {useI18n} from '@/common/i18n/useI18n.ts';
import {inTauri} from '@/common/util/TauriUtil.ts';
import {CLOSE_ACTION} from '@/common/model/Config.ts';
import {useEffect, useRef, useState, type RefObject} from 'react';
import {useServiceContainer} from '@/common/context/ServiceContext.tsx';
import {Close, CropSquare, FilterNone, Remove} from '@mui/icons-material';
import {usePlatformNativeMobile} from '@/common/context/usePlatformMobile';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';
import type {RespSocketContext} from '@/common/model/resp/RespWsContext.ts';
import {useConfigService, useSystemUtilService} from '@/common/context/useAppHooks.tsx';
import {isWindowEvent, RESP_EVENT_WINDOW} from '@/common/model/resp/event/RespEventWindow.ts';
import {Box, Button, ButtonBase, Checkbox, Dialog, FormControlLabel, Typography} from '@mui/material';
import {resolveScopedDialogProps, resolveSheetDialogProps} from '@/page/common/util/scopedDialogUtil.ts';

export function BarControls(props: { dialogContainerRef?: RefObject<HTMLElement | null> }) {
    const {t} = useI18n();
    const isTauri = inTauri();
    const container = useServiceContainer();
    const configService = useConfigService();
    const mobile = usePlatformNativeMobile();
    const systemUtilService = useSystemUtilService();
    const controlsRef = useRef<HTMLDivElement | null>(null);
    const [maximized, setMaximized] = useState<boolean>(false);
    const [closeAskOpen, setCloseAskOpen] = useState<boolean>(false);
    const [closeAskRemember, setCloseAskRemember] = useState<boolean>(false);
    const sheetDialogProps = resolveSheetDialogProps({className: 'tm-dialog-paper', sx: {width: 380}});
    const {backdropSlotProps, ...scopedDialogProps} = resolveScopedDialogProps(props.dialogContainerRef?.current);

    const handleMinimize = (): void => {
        void systemUtilService.minimizeWindow();
    };

    const handleToggleMaximize = (): void => {
        void systemUtilService.toggleMaximizeWindow();
    };

    const handleClose = (): void => {
        void systemUtilService.closeWindow();
    };

    const handleCloseAskCancel = (): void => {
        setCloseAskOpen(false);
    };

    const handleCloseAskChoice = (action: number): void => {
        setCloseAskOpen(false);
        if (closeAskRemember) {
            configService.saveCloseAction(action).catch(error => {
                console.error('[window] save close action failed', error);
            });
        }
        void systemUtilService.closeWindow(action);
    };

    useEffect(() => {
        if (!isTauri) {
            return;
        }
        void systemUtilService.getWindowMaximized().then(setMaximized);
        return container.resolve(WebSocketServiceImpl).addWsListener((context: RespSocketContext) => {
            if (!isWindowEvent(context.type)) {
                return;
            }
            if (context.type === RESP_EVENT_WINDOW.ON_MAXIMIZED_CHANGED) {
                setMaximized(!!context.payload);
            } else if (context.type === RESP_EVENT_WINDOW.ON_CLOSE_ASK) {
                if (controlsRef.current === null || controlsRef.current.closest('[inert]')) {
                    return;
                }
                setCloseAskRemember(false);
                setCloseAskOpen(true);
            }
        });
    }, [isTauri, systemUtilService, container]);

    if (!isTauri || mobile) {
        return null;
    }

    return (
        <Box ref={controlsRef} className="tm-tab-bar-controls">
            <ButtonBase aria-label={t('common.minimize')} className="tm-btn-highlight tm-win-btn" onClick={handleMinimize}>
                <Remove sx={{fontSize: 17}}/>
            </ButtonBase>
            <ButtonBase aria-label={maximized ? t('common.restore') : t('common.maximize')} className="tm-btn-highlight tm-win-btn" onClick={handleToggleMaximize}>
                {maximized ? <FilterNone sx={{fontSize: 14}}/> : <CropSquare sx={{fontSize: 17}}/>}
            </ButtonBase>
            <ButtonBase aria-label={t('common.windowClose')} className="tm-btn-highlight tm-win-btn tm-win-btn--close" onClick={handleClose}>
                <Close sx={{fontSize: 17}}/>
            </ButtonBase>
            <Dialog
                open={closeAskOpen}
                onClose={handleCloseAskCancel}
                {...sheetDialogProps}
                {...scopedDialogProps}
                disableAutoFocus={false}
                disableRestoreFocus={false}
                disableEnforceFocus={false}
                slotProps={{...sheetDialogProps.slotProps, backdrop: backdropSlotProps}}
            >
                <Box sx={{px: 2.5, pt: 2.25, pb: 0.5}}>
                    <Typography sx={{fontSize: 15, fontWeight: 700, color: 'var(--tm-text-primary)'}}>{t('common.closeAskTitle')}</Typography>
                </Box>
                <Box sx={{px: 2.5, pt: 1, pb: 1.5, minWidth: 0}}>
                    <Typography sx={{fontSize: 13, color: 'var(--tm-text-secondary)'}}>{t('common.closeAskMessage')}</Typography>
                </Box>
                <Box sx={{px: 2, py: 1.25, display: 'flex', alignItems: 'center', gap: 1, borderTop: '1px solid var(--tm-border-subtle)'}}>
                    <FormControlLabel
                        sx={{flex: 1, minWidth: 0, ml: 0, '.MuiFormControlLabel-label': {fontSize: 12, color: 'var(--tm-text-secondary)'}}}
                        control={<Checkbox size="small" checked={closeAskRemember} onChange={event => setCloseAskRemember(event.target.checked)}/>}
                        label={t('common.closeAskRemember')}
                    />
                    <Button disableRipple onClick={() => handleCloseAskChoice(CLOSE_ACTION.EXIT)} className="tm-btn tm-btn-secondary">
                        {t('set.general.closeActionExit')}
                    </Button>
                    <Button disableRipple variant="contained" onClick={() => handleCloseAskChoice(CLOSE_ACTION.TRAY)} className="tm-btn tm-btn-primary">
                        {t('set.general.closeActionTray')}
                    </Button>
                </Box>
            </Dialog>
        </Box>
    );
}
