import {Close} from '@mui/icons-material';
import {Box, ButtonBase} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNil, isNotNil} from '@/common/util/StringUtil.ts';
import type {CSSProperties, ReactNode, RefObject} from 'react';
import {usePlatformMobile} from '@/common/context/usePlatformMobile';
import {ConfirmDialog} from '@/common/components/dialog/ConfirmDialog.tsx';
import type {useInlineRevert} from '@/page/common/hooks/useInlineRevert.ts';
import {useInlineRevertEscape} from '@/page/common/hooks/useInlineRevertEscape.ts';

export function InputFrame(props: {children: ReactNode; wrapRef: RefObject<HTMLDivElement | null>; revert: ReturnType<typeof useInlineRevert>}) {
    const {t} = useI18n();
    const mobile = usePlatformMobile();
    const {children, wrapRef, revert} = props;
    const inline = !mobile && isNotNil(revert.revertItemId) && isNotNil(revert.inlineRevertBox);

    useInlineRevertEscape({wrapRef, editing: isNotNil(revert.revertItemId), submitting: revert.revertSubmitting, onCancel: revert.cancelRevert});

    return <>
        {inline ? (
            <Box ref={revert.followupRef} className="tm-agent-input-followup-slot" aria-disabled="true" style={isNotNil(revert.followupHeight) ? {height: revert.followupHeight} : undefined}>
                <Box className="tm-agent-input-followup-text">Add a follow-up</Box>
            </Box>
        ) : null}
        <Box
            ref={wrapRef}
            className={inline ? 'tm-agent-input-wrap is-inline-revert' : 'tm-agent-input-wrap'}
            style={inline && isNotNil(revert.inlineRevertBox) ? ({
                '--tm-inline-top': `${revert.inlineRevertBox.top}px`,
                '--tm-inline-left': `${revert.inlineRevertBox.left}px`,
                '--tm-inline-width': `${revert.inlineRevertBox.width}px`,
            } as CSSProperties) : undefined}
        >
            {!isNil(revert.revertItemId) ? (
                <Box className="tm-agent-input-revert">
                    <Box className="tm-agent-input-revert-text">{t('agent.input.revertHint')}</Box>
                    <ButtonBase disableRipple disabled={revert.revertSubmitting} onPointerDown={event => event.preventDefault()} onClick={revert.cancelRevert} aria-label={t('agent.input.revertCancelAria')} className="tm-agent-input-revert-close">
                        <Close sx={{fontSize: 14}} />
                    </ButtonBase>
                </Box>
            ) : null}
            <ConfirmDialog open={revert.revertAsking} title={t('agent.input.revertTitle')} message={t('agent.input.revertMessage')} confirmLabel={t('agent.input.revertConfirm')} danger autoFocus error={revert.revertError} confirmDisabled={revert.revertSubmitting} onCancel={revert.dismissRevertConfirmation} onConfirm={revert.confirmRevert} />
            {children}
        </Box>
    </>;
}
