import {useCallback, useEffect, useState} from 'react';
import {addLanguageListener, getLanguage, translate, translateFormat} from '@/common/i18n/i18n.ts';

export function useI18n() {
    const [language, setLanguageState] = useState<number>(getLanguage);

    const t = useCallback((key: string | null | undefined) => translate(key), [language]);
    const tf = useCallback((key: string, params: Record<string, string | number>) => translateFormat(key, params), [language]);

    useEffect(() => {
        return addLanguageListener(setLanguageState);
    }, []);

    return {language, t, tf};
}
