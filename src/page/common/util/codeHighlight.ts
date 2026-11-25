import {bundledLanguagesInfo, codeToTokens, type BundledLanguage, type ThemedToken} from 'shiki';

export const HIGHLIGHT_OPTIONS = {
    themes: {dark: 'one-dark-pro', light: 'github-light-high-contrast'},
    defaultColor: false,
    colorReplacements: {
        'github-light-high-contrast': {'#66707b': 'var(--tm-text-tertiary)'},
        'one-dark-pro': {'#7f848e': 'var(--tm-text-tertiary)'},
    },
} as const;

const TOKEN_CACHE_LIMIT = 120;
const NUMBERED_LINE = /^(\d+)→(.*)$/;

const tokenCache = new Map<string, ThemedToken[][]>();

const EXTRA_LANGUAGE_ALIAS: Record<string, string> = {
    cc: 'cpp',
    cxx: 'cpp',
    hpp: 'cpp',
    hxx: 'cpp',
    pwsh: 'powershell',
    psm1: 'powershell',
    psd1: 'powershell',
    cmd: 'bat',
    console: 'shellsession',
    fish: 'fish',
    ksh: 'shellscript',
    csh: 'shellscript',
    tcsh: 'shellscript',
    compose: 'docker',
    'docker-compose': 'docker',
    makefile: 'make',
    mk: 'make',
    conf: 'ini',
    cfg: 'ini',
    env: 'dotenv',
    'ssh-config': 'ssh-config',
    nginxconf: 'nginx',
    tf: 'terraform',
    hcl: 'hcl',
    proto3: 'proto',
    protobuf: 'proto',
    gql: 'graphql',
    md: 'markdown',
    mdx: 'mdx',
    mjs: 'javascript',
    cjs: 'javascript',
    mts: 'typescript',
    cts: 'typescript',
    kts: 'kotlin',
    yml: 'yaml',
    htm: 'html',
    plaintext: 'text',
    txt: 'text',
    text: 'text',
    plain: 'text',
    log: 'log',
};

function buildLanguageAlias(): Record<string, string> {
    const map: Record<string, string> = {};
    for (const info of bundledLanguagesInfo) {
        map[info.id] = info.id;
        if (!info.aliases) {
            continue;
        }
        for (const alias of info.aliases) {
            map[alias] = info.id;
        }
    }
    for (const [alias, id] of Object.entries(EXTRA_LANGUAGE_ALIAS)) {
        map[alias] = id;
    }
    return map;
}

const LANGUAGE_ALIAS = buildLanguageAlias();

export function resolveLanguage(language: string | null | undefined): string {
    if (!language) {
        return 'text';
    }
    const lower = language.toLowerCase();
    return LANGUAGE_ALIAS[lower] ?? lower;
}

export function resolveLanguageByPath(path: string | null | undefined): string {
    if (!path) {
        return 'text';
    }
    const name = path.split(/[\\/]/).pop() ?? '';
    const lower = name.toLowerCase();
    if (lower === 'dockerfile') {
        return 'docker';
    }
    if (lower === 'makefile') {
        return 'make';
    }
    const dot = lower.lastIndexOf('.');
    if (dot < 0 || dot === lower.length - 1) {
        return 'text';
    }
    const ext = lower.slice(dot + 1);
    return LANGUAGE_ALIAS[ext] ?? 'text';
}

export interface NumberedLine {
    no: number | null;
    text: string;
}

export function parseNumberedLines(text: string): NumberedLine[] | null {
    const raw = text.replace(/\r\n/g, '\n').split('\n');
    if (raw.length === 0) {
        return null;
    }
    let matched = 0;
    const lines: NumberedLine[] = raw.map(line => {
        const hit = NUMBERED_LINE.exec(line);
        if (!hit) {
            return {no: null, text: line};
        }
        matched++;
        return {no: Number(hit[1]), text: hit[2]};
    });
    if (matched === 0 || lines[0].no === null || matched * 2 < raw.length) {
        return null;
    }
    return lines;
}

function writeTokenCache(key: string, tokens: ThemedToken[][]): void {
    if (tokenCache.size >= TOKEN_CACHE_LIMIT) {
        const oldest = tokenCache.keys().next().value;
        if (oldest) {
            tokenCache.delete(oldest);
        }
    }
    tokenCache.set(key, tokens);
}

export function peekTokens(code: string, language: string): ThemedToken[][] | null {
    return tokenCache.get(`${language}\n${code}`) ?? null;
}

export async function highlightTokens(code: string, language: string): Promise<ThemedToken[][]> {
    const key = `${language}\n${code}`;
    const cached = tokenCache.get(key);
    if (cached) {
        return cached;
    }
    const lang = resolveLanguage(language) as BundledLanguage;
    let tokens: ThemedToken[][];
    try {
        tokens = (await codeToTokens(code, {lang, ...HIGHLIGHT_OPTIONS})).tokens;
    } catch {
        tokens = (await codeToTokens(code, {lang: 'text', ...HIGHLIGHT_OPTIONS})).tokens;
    }
    writeTokenCache(key, tokens);
    return tokens;
}
