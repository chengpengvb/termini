import {Refresh} from '@mui/icons-material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {Box, ButtonBase, Typography} from '@mui/material';

export interface ConnectLostProps {
    attempt: number;
    maxAttempt: number;
    countdown: number;
    onRetry: () => void;
    onStop?: () => void;
}

export function ConnectLost(props: ConnectLostProps) {
    const {t, tf} = useI18n();
    const {attempt, maxAttempt, countdown, onRetry, onStop} = props;

    const resolveText = (): string => {
        if (countdown <= 0) {
            return t('protocol.connectLost.disconnected');
        }
        return tf('protocol.connectLost.retryCountdown', {seconds: countdown, attempt, total: maxAttempt});
    };

    return (
        <Box className="tm-connect-lost">
            <Typography className="tm-connect-lost-text">{resolveText()}</Typography>
            {countdown > 0 && onStop ? (
                <ButtonBase disableRipple className="tm-connect-lost-stop" onClick={onStop}>{t('protocol.connectLost.stopAuto')}</ButtonBase>
            ) : null}
            <ButtonBase disableRipple className="tm-connect-lost-btn" onClick={onRetry} title={t('protocol.connectLost.reconnect')} aria-label={t('protocol.connectLost.reconnect')}>
                <Refresh/>
            </ButtonBase>
        </Box>
    );
}
