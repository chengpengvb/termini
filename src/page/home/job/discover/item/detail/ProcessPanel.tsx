import {useI18n} from '@/common/i18n/useI18n.ts';
import {Box, Button, Typography} from '@mui/material';
import {MessageBody} from '@/page/agent/main/body/message/MessageBody.tsx';

interface ProcessPanelProps {
    chatId: string;
    running: boolean;
    sessionId: string;
    onCancel: () => void;
}

export function ProcessPanel(props: ProcessPanelProps) {
    const {t} = useI18n();
    const {chatId, running, sessionId, onCancel} = props;

    return (
        <Box className="tm-discover-convo-section">
            <Box className="tm-discover-section-head">
                <Typography className="tm-discover-section-title">{t('home.job.discover.aiLog')}</Typography>
                {running ? <Button disableRipple className="tm-btn tm-btn-danger tm-discover-stop" onClick={onCancel}>{t('home.job.discover.stop')}</Button> : null}
            </Box>
            <Box className="tm-discover-convo">
                <MessageBody chatId={chatId} sessionId={sessionId} hideUser/>
            </Box>
        </Box>
    );
}
