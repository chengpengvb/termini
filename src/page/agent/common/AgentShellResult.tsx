import {Box} from '@mui/material';
import type {ReactNode} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {AgentCodeToolbar} from '@/page/agent/common/AgentCodeToolbar.tsx';
import {AgentShellOutput} from '@/page/agent/common/AgentShellOutput.tsx';

interface AgentShellResultProps {
    content: string;
    running: boolean;
    active: boolean;
}

export function AgentShellResult(props: AgentShellResultProps): ReactNode {
    const {content, running, active} = props;
    const {t} = useI18n();

    return (
        <Box className="tm-agent-code-block tm-agent-shell-result">
            <AgentCodeToolbar label={t('agent.tool.shellOutput')} copyText={content}/>
            {running ? (
                <AgentShellOutput content={content} active={active}/>
            ) : (
                <Box component="pre" className="tm-agent-shell-output-static">
                    {content}
                </Box>
            )}
        </Box>
    );
}
