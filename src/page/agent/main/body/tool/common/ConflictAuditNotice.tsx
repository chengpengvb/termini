import {Box} from '@mui/material';
import {isEmpty} from '@/common/util/StringUtil.ts';
import {WarningAmberOutlined} from '@mui/icons-material';

interface ConflictAuditNoticeProps {
    reason: string | null;
}

export function ConflictAuditNotice(props: ConflictAuditNoticeProps) {
    const {reason} = props;

    if (isEmpty(reason)) {
        return null;
    }

    return (
        <Box className="tm-agent-tool-audit">
            <WarningAmberOutlined className="tm-agent-tool-audit-icon"/>
            <Box component="span" className="tm-agent-tool-audit-text">{reason}</Box>
        </Box>
    );
}
