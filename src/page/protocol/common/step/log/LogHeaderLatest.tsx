import {Box, Typography} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {LogEntry} from '@/page/common/dto/terminal/LogEntry.ts';
import {LogEntryItem} from '@/page/protocol/common/step/log/LogEntryItem.tsx';

export interface LogHeaderLatestProps {
    logs: LogEntry[];
}

export function LogHeaderLatest(props: LogHeaderLatestProps) {
    const {logs} = props;
    const {t} = useI18n();
    if (logs.length === 0) {
        return <Typography className="tm-protocol-log-empty">{t('protocol.log.empty')}</Typography>;
    }
    const latest = logs[logs.length - 1];
    return (
        <Box className="tm-protocol-log-latest">
            <LogEntryItem log={latest}/>
        </Box>
    );
}
