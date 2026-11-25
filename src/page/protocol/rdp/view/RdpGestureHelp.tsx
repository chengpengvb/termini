import {Box, ButtonBase} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {Mouse, TouchApp} from '@mui/icons-material';
import {RDP_MOBILE_QUALITIES, type RdpMobileQuality} from '@/page/common/util/rdpMobilePreferences.ts';

export interface RdpGestureHelpProps {
    touchMode: boolean;
    onClose: () => void;
    onTouchModeChange: (touchMode: boolean) => void;
    quality: RdpMobileQuality;
    onQualityChange: (quality: RdpMobileQuality) => void;
    onFit: () => void;
    onActualSize: () => void;
    onScroll: (direction: number) => void;
}

export function RdpGestureHelp(props: RdpGestureHelpProps) {
    const {t} = useI18n();
    const {touchMode, onClose, onTouchModeChange, quality, onQualityChange, onFit, onActualSize, onScroll} = props;
    const qualityLabels = {smooth: 'qualitySmooth', balanced: 'qualityBalanced', sharp: 'qualitySharp'};
    const items = touchMode
        ? [
            [t('protocol.rdp.tapOne'), t('protocol.rdp.leftClick')],
            [t('protocol.rdp.tapTwo'), t('protocol.rdp.rightClick')],
            [t('protocol.rdp.tapDoubleMove'), t('protocol.rdp.mouseDrag')],
            [t('protocol.rdp.moveOne'), t('protocol.rdp.canvasMove')],
            [t('protocol.rdp.moveTwo'), t('protocol.rdp.canvasMove')],
            [t('protocol.rdp.pinchZoom'), t('protocol.rdp.canvasZoom')],
        ]
        : [
            [t('protocol.rdp.tapOne'), t('protocol.rdp.leftClick')],
            [t('protocol.rdp.tapTwo'), t('protocol.rdp.rightClick')],
            [t('protocol.rdp.tapDoubleMove'), t('protocol.rdp.mouseDrag')],
            [t('protocol.rdp.moveOne'), t('protocol.rdp.pointerMove')],
            [t('protocol.rdp.moveTwo'), t('protocol.rdp.mouseWheel')],
            [t('protocol.rdp.pinchZoom'), t('protocol.rdp.canvasZoom')],
        ];

    return (
        <Box className="tm-rdp-gesture" onPointerDown={event => event.stopPropagation()} onMouseDown={event => event.preventDefault()}>
            <ButtonBase disableRipple className="tm-rdp-gesture-mask" onClick={onClose} aria-label={t('common.close')}/>
            <Box className="tm-rdp-gesture-sheet">
                <Box className="tm-rdp-gesture-switch">
                    <ButtonBase disableRipple className={`tm-rdp-gesture-mode${!touchMode ? ' is-active' : ''}`} onClick={() => onTouchModeChange(false)}>
                        <Mouse fontSize="inherit"/>
                        {t('protocol.rdp.mouseMode')}
                    </ButtonBase>
                    <ButtonBase disableRipple className={`tm-rdp-gesture-mode${touchMode ? ' is-active' : ''}`} onClick={() => onTouchModeChange(true)}>
                        <TouchApp fontSize="inherit"/>
                        {t('protocol.rdp.touchMode')}
                    </ButtonBase>
                </Box>
                <Box className="tm-rdp-gesture-actions">
                    <ButtonBase className="tm-rdp-gesture-mode" onClick={() => { onFit(); onClose(); }}>{t('protocol.rdp.fitView')}</ButtonBase>
                    <ButtonBase className="tm-rdp-gesture-mode" onClick={() => { onActualSize(); onClose(); }}>{t('protocol.rdp.actualPixels')}</ButtonBase>
                    <ButtonBase className="tm-rdp-gesture-mode" onClick={() => onScroll(1)}>{t('protocol.rdp.scrollUp')}</ButtonBase>
                    <ButtonBase className="tm-rdp-gesture-mode" onClick={() => onScroll(-1)}>{t('protocol.rdp.scrollDown')}</ButtonBase>
                </Box>
                <Box className="tm-rdp-gesture-quality">
                    <Box>{t('protocol.rdp.qualityTitle')}</Box>
                    <Box className="tm-rdp-gesture-actions">
                        {RDP_MOBILE_QUALITIES.map(value => <ButtonBase key={value} className={'tm-rdp-gesture-mode' + (quality === value ? ' is-active' : '')} onClick={() => onQualityChange(value)}>{t('protocol.rdp.' + qualityLabels[value])}</ButtonBase>)}
                    </Box>
                    <Box className="tm-rdp-gesture-quality-hint">{t('protocol.rdp.qualityHint')}</Box>
                </Box>
                <Box className="tm-rdp-gesture-grid">
                    {items.map(item => (
                        <Box key={item[0]} className="tm-rdp-gesture-item">
                            <Box className="tm-rdp-gesture-from">{item[0]}</Box>
                            <Box className="tm-rdp-gesture-to">{item[1]}</Box>
                        </Box>
                    ))}
                </Box>
            </Box>
        </Box>
    );
}
