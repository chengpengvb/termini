import {codeToHtml} from 'shiki';
import {Box} from '@mui/material';
import {isNil} from '@/common/util/StringUtil.ts';
import {useEffect, useState, type ReactNode} from 'react';
import {AgentCodeToolbar} from '@/page/agent/common/AgentCodeToolbar.tsx';
import {HIGHLIGHT_OPTIONS, resolveLanguage} from '@/page/common/util/codeHighlight.ts';

const HIGHLIGHT_CACHE_LIMIT = 200;

const highlightCache = new Map<string, string>();

interface AgentCodeBlockProps {
    code: string;
    language: string | null;
}

function buildCacheKey(code: string, language: string): string {
    return `${language}\n${code}`;
}

function writeHighlightCache(key: string, html: string): void {
    if (highlightCache.size >= HIGHLIGHT_CACHE_LIMIT) {
        const oldest = highlightCache.keys().next().value;
        if (oldest) {
            highlightCache.delete(oldest);
        }
    }
    highlightCache.set(key, html);
}

export function AgentCodeBlock(props: AgentCodeBlockProps): ReactNode {
    const {code, language} = props;
    const [html, setHtml] = useState<string | null>(() => highlightCache.get(buildCacheKey(code, language ?? '')) ?? null);

    function resolveLabel(): string {
        if (!language) {
            return 'text';
        }
        return language;
    }

    useEffect(() => {
        let cancelled = false;
        const key = buildCacheKey(code, language ?? '');
        const cached = highlightCache.get(key);
        if (cached) {
            setHtml(cached);
            return;
        }
        const lang = resolveLanguage(language);
        const apply = (result: string): void => {
            writeHighlightCache(key, result);
            if (cancelled) {
                return;
            }
            setHtml(result);
        };
        codeToHtml(code, {lang, ...HIGHLIGHT_OPTIONS})
            .then(apply)
            .catch(() => {
                codeToHtml(code, {lang: 'text', ...HIGHLIGHT_OPTIONS}).then(apply);
            });
        return () => {
            cancelled = true;
        };
    }, [code, language]);

    return (
        <Box className="tm-agent-code-block">
            <AgentCodeToolbar label={resolveLabel()} copyText={code}/>
            {isNil(html) ? (
                <Box component="pre" className="tm-agent-code-fallback">
                    <Box component="code">{code}</Box>
                </Box>
            ) : (
                <Box className="tm-agent-code-body" dangerouslySetInnerHTML={{__html: html}}/>
            )}
        </Box>
    );
}
