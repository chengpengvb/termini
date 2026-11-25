import {isNil} from '@/common/util/StringUtil.ts';
import {isChatPreviewFile} from '@/page/common/util/fileTypeUtil.ts';

export const FILE_URL_PREFIX = 'file://';

const LOCAL_PATH_PATTERN = /[A-Za-z]:[\\/][^\s"'<>|?*:]+|(?<![\w:/])\/(?:[^\s"'<>|?*:()]+\/)*[^\s"'<>|?*:()]+/g;
const LOCAL_PATH_FULL_PATTERN = /^(?:[A-Za-z]:[\\/]|\/)[^\s"'<>|?*:]+$/;
const WINDOWS_PATH_HEAD_PATTERN = /^[A-Za-z]:[\\/]/;
const TRAILING_PUNCTUATION_PATTERN = /[),.;'"\]。,;)】'"]+$/;

const TEXT_FILE_EXTENSIONS = new Set([
    'txt', 'md', 'markdown', 'log', 'json', 'jsonl', 'yaml', 'yml', 'toml', 'ini', 'conf', 'cfg', 'properties', 'env',
    'csv', 'tsv', 'xml', 'html', 'htm', 'css', 'scss', 'less', 'svg',
    'js', 'jsx', 'ts', 'tsx', 'mjs', 'cjs', 'vue',
    'py', 'rs', 'go', 'java', 'kt', 'c', 'h', 'cpp', 'hpp', 'cs', 'php', 'rb', 'swift', 'lua',
    'sql', 'sh', 'bash', 'zsh', 'ps1', 'psm1', 'bat', 'cmd', 'gradle',
]);

interface MdNode {
    type: string;
    value?: string;
    url?: string;
    children?: MdNode[];
}

function trimTrailingPunctuation(value: string): string {
    return value.replace(TRAILING_PUNCTUATION_PATTERN, '');
}

function hasTextFileExtension(path: string): boolean {
    const parts = path.split(/[\\/]/);
    const name = parts[parts.length - 1];
    const dot = name.lastIndexOf('.');
    if (dot <= 0) {
        return false;
    }
    return TEXT_FILE_EXTENSIONS.has(name.slice(dot + 1).toLowerCase());
}

function buildFileLinkUrl(path: string): string {
    const segments = path.replace(/\\/g, '/').split('/');
    return `${FILE_URL_PREFIX}${path.startsWith('/') ? '' : '/'}${segments.map((segment) => encodeURIComponent(segment)).join('/')}`;
}

export function resolveFileLinkPath(href: string | undefined): string | null {
    if (isNil(href)) {
        return null;
    }
    const nativeWindows = /^[A-Za-z]:(?:[\\/]|%5c|%2f)/i.test(href);
    if (!nativeWindows && !href.startsWith(FILE_URL_PREFIX) && !href.startsWith('/')) {
        return null;
    }
    let rest = href.startsWith(FILE_URL_PREFIX) ? href.slice(FILE_URL_PREFIX.length) : href;
    try {
        rest = decodeURIComponent(rest);
    } catch {
        return null;
    }
    const windows = rest.replace(/^\/+/, '');
    if (WINDOWS_PATH_HEAD_PATTERN.test(windows)) {
        return windows.replace(/\//g, '\\');
    }
    return rest.startsWith('/') ? rest : null;
}

export function matchTextFilePath(text: string): string | null {
    const trimmed = trimTrailingPunctuation(text.trim());
    if (!LOCAL_PATH_FULL_PATTERN.test(trimmed)) {
        return null;
    }
    if (!hasTextFileExtension(trimmed) && !isChatPreviewFile(trimmed)) {
        return null;
    }
    return trimmed;
}

function splitTextNode(value: string): MdNode[] | null {
    LOCAL_PATH_PATTERN.lastIndex = 0;
    let match: RegExpExecArray | null;
    let cursor = 0;
    const parts: MdNode[] = [];
    while ((match = LOCAL_PATH_PATTERN.exec(value)) !== null) {
        const filePath = trimTrailingPunctuation(match[0]);
        if (!hasTextFileExtension(filePath) && !isChatPreviewFile(filePath)) {
            continue;
        }
        if (match.index > cursor) {
            parts.push({type: 'text', value: value.slice(cursor, match.index)});
        }
        parts.push({
            type: 'link',
            url: buildFileLinkUrl(filePath),
            children: [{type: 'text', value: filePath}],
        });
        cursor = match.index + filePath.length;
    }
    if (parts.length === 0) {
        return null;
    }
    if (cursor < value.length) {
        parts.push({type: 'text', value: value.slice(cursor)});
    }
    return parts;
}

function walkNode(node: MdNode): void {
    const children = node.children;
    if (isNil(children)) {
        return;
    }
    const next: MdNode[] = [];
    let changed = false;
    for (const child of children) {
        if (child.type === 'link' || child.type === 'linkReference') {
            next.push(child);
            continue;
        }
        if (child.type === 'text') {
            const parts = splitTextNode(child.value ?? '');
            if (isNil(parts)) {
                next.push(child);
            } else {
                next.push(...parts);
                changed = true;
            }
            continue;
        }
        walkNode(child);
        next.push(child);
    }
    if (changed) {
        node.children = next;
    }
}

export function remarkFilePath() {
    return (tree: unknown) => {
        walkNode(tree as MdNode);
    };
}
