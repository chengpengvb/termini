import {Box} from '@mui/material';
import type {ThemedToken} from 'shiki';
import type {CSSProperties, ReactNode} from 'react';
import {isNotNil} from '@/common/util/StringUtil.ts';
import type {NumberedLine} from '@/page/common/util/codeHighlight.ts';
import {useHighlightTokens} from '@/page/common/hooks/useHighlightTokens.ts';

interface AgentCodeLinesProps {
    lines: NumberedLine[];
    language: string;
}

export function renderTokens(tokens: ThemedToken[] | undefined, fallback: string): ReactNode {
    if (!tokens) {
        return fallback;
    }
    return tokens.map((token, index) => (
        <Box key={index} component="span" className="tm-code-token" style={token.htmlStyle as CSSProperties}>
            {token.content}
        </Box>
    ));
}

export function AgentCodeLines(props: AgentCodeLinesProps) {
    const {lines, language} = props;
    const code = lines.map(line => line.text).join('\n');
    const tokens = useHighlightTokens(code, language);
    const aligned = isNotNil(tokens) && tokens.length === lines.length ? tokens : null;
    const showNumbers = lines.some(line => line.no !== null);

    return (
        <Box className="tm-agent-code-lines">
            {lines.map((line, index) => (
                <Box key={index} className="tm-agent-code-line">
                    {showNumbers ? (
                        <Box component="span" className="tm-agent-code-lineno">
                            {line.no ?? ''}
                        </Box>
                    ) : null}
                    <Box component="span" className="tm-agent-code-text">
                        {renderTokens(aligned?.[index], line.text)}
                    </Box>
                </Box>
            ))}
        </Box>
    );
}
