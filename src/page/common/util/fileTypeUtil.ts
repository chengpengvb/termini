import {Icon} from '@iconify/react';
import iconGit from '@iconify-icons/mdi/git';
import iconXml from '@iconify-icons/mdi/xml';
import iconVue from '@iconify-icons/mdi/vuejs';
import type {IconifyIcon} from '@iconify/types';
import iconReact from '@iconify-icons/mdi/react';
import iconC from '@iconify-icons/mdi/language-c';
import iconR from '@iconify-icons/mdi/language-r';
import iconSql from '@iconify-icons/mdi/database';
import iconDocker from '@iconify-icons/mdi/docker';
import iconShell from '@iconify-icons/mdi/console';
import {createElement, type ReactNode} from 'react';
import iconGo from '@iconify-icons/mdi/language-go';
import iconJson from '@iconify-icons/mdi/code-json';
import iconKey from '@iconify-icons/mdi/key-outline';
import iconCpp from '@iconify-icons/mdi/language-cpp';
import iconLua from '@iconify-icons/mdi/language-lua';
import iconPhp from '@iconify-icons/mdi/language-php';
import iconCss from '@iconify-icons/mdi/language-css3';
import iconConfig from '@iconify-icons/mdi/cog-outline';
import iconJava from '@iconify-icons/mdi/language-java';
import iconRuby from '@iconify-icons/mdi/language-ruby';
import iconRust from '@iconify-icons/mdi/language-rust';
import iconHtml from '@iconify-icons/mdi/language-html5';
import iconSwift from '@iconify-icons/mdi/language-swift';
import iconPowershell from '@iconify-icons/mdi/powershell';
import iconText from '@iconify-icons/mdi/text-box-outline';
import iconCode from '@iconify-icons/mdi/file-code-outline';
import iconCsharp from '@iconify-icons/mdi/language-csharp';
import iconKotlin from '@iconify-icons/mdi/language-kotlin';
import iconPython from '@iconify-icons/mdi/language-python';
import iconHaskell from '@iconify-icons/mdi/language-haskell';
import iconMarkdown from '@iconify-icons/mdi/language-markdown';
import iconJavascript from '@iconify-icons/mdi/language-javascript';
import iconTypescript from '@iconify-icons/mdi/language-typescript';
import iconCertificate from '@iconify-icons/mdi/file-certificate-outline';

const ICON_BY_EXTENSION: Record<string, IconifyIcon> = {
    java: iconJava,
    py: iconPython,
    pyi: iconPython,
    js: iconJavascript,
    mjs: iconJavascript,
    cjs: iconJavascript,
    jsx: iconReact,
    ts: iconTypescript,
    mts: iconTypescript,
    cts: iconTypescript,
    tsx: iconReact,
    vue: iconVue,
    html: iconHtml,
    htm: iconHtml,
    css: iconCss,
    scss: iconCss,
    sass: iconCss,
    less: iconCss,
    md: iconMarkdown,
    markdown: iconMarkdown,
    json: iconJson,
    json5: iconJson,
    jsonc: iconJson,
    xml: iconXml,
    xsd: iconXml,
    xsl: iconXml,
    svg: iconXml,
    plist: iconXml,
    rs: iconRust,
    go: iconGo,
    c: iconC,
    h: iconC,
    cpp: iconCpp,
    cc: iconCpp,
    cxx: iconCpp,
    hpp: iconCpp,
    hh: iconCpp,
    cs: iconCsharp,
    php: iconPhp,
    rb: iconRuby,
    kt: iconKotlin,
    kts: iconKotlin,
    swift: iconSwift,
    lua: iconLua,
    r: iconR,
    hs: iconHaskell,
    sh: iconShell,
    bash: iconShell,
    zsh: iconShell,
    fish: iconShell,
    bat: iconShell,
    cmd: iconShell,
    ps1: iconPowershell,
    psm1: iconPowershell,
    psd1: iconPowershell,
    sql: iconSql,
    yml: iconConfig,
    yaml: iconConfig,
    toml: iconConfig,
    ini: iconConfig,
    conf: iconConfig,
    cfg: iconConfig,
    env: iconConfig,
    properties: iconConfig,
    dockerfile: iconDocker,
    gitignore: iconGit,
    gitattributes: iconGit,
    pem: iconCertificate,
    crt: iconCertificate,
    cer: iconCertificate,
    pub: iconKey,
    txt: iconText,
    log: iconText,
    csv: iconText,
};

const ICON_BY_NAME: Record<string, IconifyIcon> = {
    dockerfile: iconDocker,
    makefile: iconCode,
    'cmakelists.txt': iconCode,
    '.gitignore': iconGit,
    '.gitattributes': iconGit,
    '.gitmodules': iconGit,
    '.env': iconConfig,
    '.editorconfig': iconConfig,
};

const NOT_TEXT_EXTENSIONS: ReadonlySet<string> = new Set([
    'pdf', 'doc', 'docx', 'xls', 'xlsx', 'ppt', 'pptx', 'odt', 'ods', 'odp', 'rtf', 'pages', 'numbers', 'key',
    'zip', 'rar', '7z', 'tar', 'gz', 'tgz', 'bz2', 'xz', 'zst', 'jar', 'war', 'ear',
    'png', 'jpg', 'jpeg', 'gif', 'bmp', 'webp', 'ico', 'icns', 'tif', 'tiff', 'heic', 'psd',
    'mp3', 'wav', 'flac', 'aac', 'ogg', 'm4a', 'mp4', 'mov', 'avi', 'mkv', 'webm', 'wmv',
    'exe', 'dll', 'so', 'dylib', 'bin', 'iso', 'dmg', 'msi', 'apk', 'ipa', 'deb', 'rpm', 'class', 'o', 'obj', 'pyc', 'wasm',
    'ttf', 'otf', 'woff', 'woff2', 'eot',
    'db', 'sqlite', 'sqlite3', 'mdb', 'accdb', 'parquet',
]);

export function resolveFileName(path: string): string {
    const parts = path.split(/[\\/]/).filter(part => part.length > 0);
    if (parts.length === 0) {
        return path;
    }
    return parts[parts.length - 1];
}

const CHAT_PREVIEW_EXTENSIONS: ReadonlySet<string> = new Set([
    'png', 'jpg', 'jpeg', 'gif', 'bmp', 'webp', 'svg', 'ico', 'tif', 'tiff', 'avif', 'heic', 'heif',
    'mp4', 'mov', 'webm', 'mkv', 'avi', 'm4v', 'mp3', 'wav', 'flac', 'aac', 'ogg', 'm4a', 'opus',
    'pdf', 'doc', 'docx', 'xls', 'xlsx', 'ppt', 'pptx', 'odt', 'ods', 'odp', 'rtf',
    'txt', 'md', 'markdown', 'json', 'csv', 'tsv', 'xml', 'html', 'htm', 'log',
]);

export function isChatPreviewFile(path: string): boolean {
    return CHAT_PREVIEW_EXTENSIONS.has(resolveExtension(resolveFileName(path)));
}

function resolveExtension(fileName: string): string {
    const index = fileName.lastIndexOf('.');
    if (index < 0 || index === fileName.length - 1) {
        return '';
    }
    return fileName.slice(index + 1).toLowerCase();
}

export function isTextEditableFile(path: string): boolean {
    return !NOT_TEXT_EXTENSIONS.has(resolveExtension(resolveFileName(path)));
}

export function resolveFileTypeIcon(path: string, className: string): ReactNode {
    const fileName = resolveFileName(path);
    const byName = ICON_BY_NAME[fileName.toLowerCase()];
    if (byName) {
        return createElement(Icon, {icon: byName, className, inline: false});
    }
    const byExtension = ICON_BY_EXTENSION[resolveExtension(fileName)];
    if (byExtension) {
        return createElement(Icon, {icon: byExtension, className, inline: false});
    }
    return createElement(Icon, {icon: iconCode, className, inline: false});
}
