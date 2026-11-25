import {Box, Typography} from '@mui/material';
import {LogEntry} from '@/page/common/dto/terminal/LogEntry.ts';
import {formatLogTime, LOG_LEVEL_CLASS, LOG_LEVEL_TEXT} from '@/page/common/util/logHelpers.ts';

export interface LogEntryItemProps {
    log: LogEntry;
}

export function LogEntryItem(props: LogEntryItemProps) {
    const {log} = props;
    const stateClass = LOG_LEVEL_CLASS[log.level];
    return (
        <Box className="tm-protocol-log-entry">
            <Typography component="span" className="tm-protocol-log-time">{formatLogTime(log.timestamp)}</Typography>
            <Typography component="span" className={`tm-protocol-log-level ${stateClass}`}>{LOG_LEVEL_TEXT[log.level]}</Typography>
            <Typography component="span" className={`tm-protocol-log-text ${stateClass}`}>{log.text}</Typography>
        </Box>
    );
}
