import {useI18n} from '@/common/i18n/useI18n.ts';
import {Box, IconButton, Tooltip} from '@mui/material';
import {Check, ContentCopy} from '@mui/icons-material';
import {useEffect, useState, type ReactNode} from 'react';

interface AgentCodeToolbarProps {
    label: string;
    copyText: string;
}

export function AgentCodeToolbar(props: AgentCodeToolbarProps): ReactNode {
    const {label, copyText} = props;
    const {t} = useI18n();
    const [copied, setCopied] = useState(false);

    async function handleCopy(): Promise<void> {
        await navigator.clipboard.writeText(copyText);
        setCopied(true);
    }

    useEffect(() => {
        if (!copied) {
            return;
        }
        const timer = window.setTimeout(() => {
            setCopied(false);
        }, 1500);
        return () => {
            window.clearTimeout(timer);
        };
    }, [copied]);

    const copyTooltip = copied ? t('agent.copy.copied') : t('agent.copy.code');

    return (
        <Box className="tm-agent-code-toolbar">
            <Box component="span" className="tm-agent-code-lang">{label}</Box>
            <Tooltip title={copyTooltip}>
                <IconButton size="small" className="tm-agent-code-copy" onClick={() => {
                    void handleCopy();
                }} aria-label={copyTooltip}>
                    {copied ? <Check sx={{fontSize: 15}}/> : <ContentCopy sx={{fontSize: 15}}/>}
                </IconButton>
            </Tooltip>
        </Box>
    );
}
