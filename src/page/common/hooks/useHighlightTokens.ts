import type {ThemedToken} from 'shiki';
import {useEffect, useState} from 'react';
import {highlightTokens, peekTokens} from '@/page/common/util/codeHighlight.ts';

export function useHighlightTokens(code: string | null, language: string): ThemedToken[][] | null {
    const [tokens, setTokens] = useState<ThemedToken[][] | null>(() => (code === null ? null : peekTokens(code, language)));

    useEffect(() => {
        if (code === null) {
            setTokens(null);
            return;
        }
        const cached = peekTokens(code, language);
        if (cached) {
            setTokens(cached);
            return;
        }
        let cancelled = false;
        highlightTokens(code, language).then(result => {
            if (!cancelled) {
                setTokens(result);
            }
        }).catch(() => {
            if (!cancelled) {
                setTokens(null);
            }
        });
        return () => {
            cancelled = true;
        };
    }, [code, language]);

    return tokens;
}
