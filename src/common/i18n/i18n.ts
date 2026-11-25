import {DE_DE} from './catalog/deDe.ts';
import {EN_US} from './catalog/enUs.ts';
import {ES_ES} from './catalog/esEs.ts';
import {FR_FR} from './catalog/frFr.ts';
import {HI_IN} from './catalog/hiIn.ts';
import {IT_IT} from './catalog/itIt.ts';
import {JA_JP} from './catalog/jaJp.ts';
import {KO_KR} from './catalog/koKr.ts';
import {NL_NL} from './catalog/nlNl.ts';
import {PT_PT} from './catalog/ptPt.ts';
import {ZH_CN} from './catalog/zhCn.ts';
import {ZH_TW} from './catalog/zhTw.ts';
import {isEmpty} from '@/common/util/StringUtil.ts';
import type {Catalog, CatalogNode} from './Catalog.ts';
import {SYSTEM_LANGUAGE, resolveSystemLanguageFromLocale} from '@/common/model/Config.ts';

const CATALOGS: Record<number, Catalog> = {
    [SYSTEM_LANGUAGE.ZH_CN]: ZH_CN,
    [SYSTEM_LANGUAGE.EN_US]: EN_US,
    [SYSTEM_LANGUAGE.JA_JP]: JA_JP,
    [SYSTEM_LANGUAGE.KO_KR]: KO_KR,
    [SYSTEM_LANGUAGE.DE_DE]: DE_DE,
    [SYSTEM_LANGUAGE.FR_FR]: FR_FR,
    [SYSTEM_LANGUAGE.ES_ES]: ES_ES,
    [SYSTEM_LANGUAGE.IT_IT]: IT_IT,
    [SYSTEM_LANGUAGE.PT_PT]: PT_PT,
    [SYSTEM_LANGUAGE.NL_NL]: NL_NL,
    [SYSTEM_LANGUAGE.ZH_TW]: ZH_TW,
    [SYSTEM_LANGUAGE.HI_IN]: HI_IN,
};

let language: number = detectOsLanguage();
const listeners = new Set<(next: number) => void>();

function detectOsLanguage(): number {
    return resolveSystemLanguageFromLocale(navigator.language);
}

function lookup(catalog: Catalog, key: string): string | undefined {
    const parts = key.split('.');
    let current: CatalogNode = catalog;
    for (const part of parts) {
        if (!(current instanceof Object)) {
            return undefined;
        }
        const next: CatalogNode | undefined = current[part];
        if (next === undefined) {
            return undefined;
        }
        current = next;
    }
    if (current instanceof Object) {
        return undefined;
    }
    return current;
}

export function getLanguage(): number {
    return language;
}

export function setLanguage(next: number | null | undefined): void {
    const resolved = next != null && CATALOGS[next] != null ? next : detectOsLanguage();
    if (resolved === language) {
        return;
    }
    language = resolved;
    for (const listener of listeners) {
        listener(language);
    }
}

export function addLanguageListener(listener: (next: number) => void): () => void {
    listeners.add(listener);
    return () => {
        listeners.delete(listener);
    };
}

export function translate(key: string | null | undefined): string {
    if (isEmpty(key)) {
        return '';
    }
    const catalog = CATALOGS[language] ?? ZH_CN;
    const text = lookup(catalog, key);
    if (text !== undefined) {
        return text;
    }
    const fallback = lookup(ZH_CN, key);
    if (fallback !== undefined) {
        return fallback;
    }
    return key;
}

export function translateFormat(key: string, params: Record<string, string | number>): string {
    let text = translate(key);
    for (const [name, value] of Object.entries(params)) {
        text = text.split(`{${name}}`).join(String(value));
    }
    return text;
}
