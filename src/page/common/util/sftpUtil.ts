import React from 'react';
import {isNil} from "@/common/util/StringUtil.ts";
import {PhysicalPosition} from '@tauri-apps/api/dpi';
import {FILE_KIND, FileEntry} from "@/common/model/dto/FileEntry.ts";
import {SftpTransferEvent} from "@/common/model/req/event/WsSftp.ts";
import {Article, AudioFile, Code, Css, DataObject, Description, FolderZip, FontDownload, Html, Image, InsertDriveFile, Settings, Slideshow, Storage, TableChart, VideoFile} from '@mui/icons-material';

export const SFTP_ENTRY_SORT_COLUMN = {
    NAME: 1,
    DATE: 2,
    SIZE: 3,
    KIND: 4,
} as const;

export const SFTP_ENTRY_SORT_ORDER = {
    ASC: 1,
    DESC: 2,
} as const;

export const SFTP_ENTRY_ROW_HEIGHT = 48;
export const SFTP_ENTRY_ROW_HEIGHT_MOBILE = 56;

export function resolveSftpEntryRowHeight(mobileViewport: boolean): number {
    return mobileViewport ? SFTP_ENTRY_ROW_HEIGHT_MOBILE : SFTP_ENTRY_ROW_HEIGHT;
}

export function resolveDragSelectNames(entries: FileEntry[], startContentY: number, currentContentY: number, rowHeight: number): string[] {
    const top = Math.min(startContentY, currentContentY);
    const bottom = Math.max(startContentY, currentContentY);
    let startIndex = Math.floor(top / rowHeight);
    let endIndex = Math.floor(bottom / rowHeight);
    if (startIndex < 0) {
        startIndex = 0;
    }
    if (endIndex >= entries.length) {
        endIndex = entries.length - 1;
    }
    const names: string[] = [];
    for (let index = startIndex; index <= endIndex; index += 1) {
        const entry = entries[index];
        if (!entry) {
            continue;
        }
        if (entry.kind === FILE_KIND.PARENT) {
            continue;
        }
        names.push(entry.name);
    }
    return names;
}

const SFTP_NAME_COLLATOR = new Intl.Collator(undefined, {
    usage: 'sort',
    numeric: true,
    sensitivity: 'base',
    ignorePunctuation: false,
});

const SFTP_NAME_COLLATOR_TIEBREAK = new Intl.Collator(undefined, {
    usage: 'sort',
    numeric: true,
    sensitivity: 'variant',
    ignorePunctuation: false,
});

function compareSftpEntryName(left: FileEntry, right: FileEntry): number {
    const primary = SFTP_NAME_COLLATOR.compare(left.name, right.name);
    if (primary !== 0) {
        return primary;
    }
    return SFTP_NAME_COLLATOR_TIEBREAK.compare(left.name, right.name);
}

function compareSftpEntryDate(left: FileEntry, right: FileEntry): number {
    if (isNil(left.mtime) && isNil(right.mtime)) {
        return 0;
    }
    if (isNil(left.mtime)) {
        return 1;
    }
    if (isNil(right.mtime)) {
        return -1;
    }
    return left.mtime - right.mtime;
}

function compareSftpEntrySize(left: FileEntry, right: FileEntry): number {
    if (isNil(left.size) && isNil(right.size)) {
        return 0;
    }
    if (isNil(left.size)) {
        return 1;
    }
    if (isNil(right.size)) {
        return -1;
    }
    return left.size - right.size;
}

function compareSftpEntryKind(left: FileEntry, right: FileEntry): number {
    const leftRank = resolveSftpEntryKindRank(left.kind);
    const rightRank = resolveSftpEntryKindRank(right.kind);
    return leftRank - rightRank;
}

function resolveSftpEntryGroupRank(kind: string): number {
    if (kind === FILE_KIND.DIRECTORY) {
        return 0;
    }
    if (kind === FILE_KIND.SYMLINK) {
        return 1;
    }
    if (kind === FILE_KIND.FILE) {
        return 2;
    }
    return 3;
}

function resolveSftpEntryKindRank(kind: string): number {
    if (kind === FILE_KIND.DIRECTORY) {
        return 0;
    }
    if (kind === FILE_KIND.SYMLINK) {
        return 1;
    }
    if (kind === FILE_KIND.FILE) {
        return 2;
    }
    return 3;
}

function compareSftpEntries(left: FileEntry, right: FileEntry, column: number, order: number): number {
    const groupResult = resolveSftpEntryGroupRank(left.kind) - resolveSftpEntryGroupRank(right.kind);
    if (groupResult !== 0) {
        return groupResult;
    }

    let result = 0;
    if (column === SFTP_ENTRY_SORT_COLUMN.NAME) {
        result = compareSftpEntryName(left, right);
    } else if (column === SFTP_ENTRY_SORT_COLUMN.DATE) {
        result = compareSftpEntryDate(left, right);
    } else if (column === SFTP_ENTRY_SORT_COLUMN.SIZE) {
        result = compareSftpEntrySize(left, right);
    } else if (column === SFTP_ENTRY_SORT_COLUMN.KIND) {
        result = compareSftpEntryKind(left, right);
    }

    if (result === 0) {
        result = compareSftpEntryName(left, right);
    }

    if (order === SFTP_ENTRY_SORT_ORDER.DESC) {
        return -result;
    }
    return result;
}

export function sortSftpEntries(entries: FileEntry[], column: number, order: number): FileEntry[] {
    const parentEntry = entries.find(entry => entry.kind === FILE_KIND.PARENT);
    const sortableEntries = entries.filter(entry => entry.kind !== FILE_KIND.PARENT);
    const sortedEntries = [...sortableEntries].sort((left, right) => {
        return compareSftpEntries(left, right, column, order);
    });
    if (parentEntry) {
        return [parentEntry, ...sortedEntries];
    }
    return sortedEntries;
}

export function formatPermissions(kind: string, permissions: number | null | undefined): string {
    if (kind === FILE_KIND.PARENT) {
        return '';
    }
    if (isNil(permissions)) {
        return '';
    }
    const typeChar = kind === FILE_KIND.DIRECTORY ? 'd' : kind === FILE_KIND.SYMLINK ? 'l' : '-';
    const p = permissions & 0o7777;
    const sticky = (p & 0o1000) > 0;
    const sgid = (p & 0o2000) > 0;
    const suid = (p & 0o4000) > 0;
    const rwx = (n: number, special: boolean, sc: string) => {
        const r = n & 4 ? 'r' : '-';
        const w = n & 2 ? 'w' : '-';
        const x = n & 1 ? (special ? sc : 'x') : special ? sc.toUpperCase() : '-';
        return r + w + x;
    };
    return typeChar + rwx((p >> 6) & 7, suid, 's') + rwx((p >> 3) & 7, sgid, 's') + rwx(p & 7, sticky, 't');
}

export function formatByteSize(bytes: number): string {
    if (bytes < 1024) {
        return `${bytes} B`;
    }
    if (bytes < 1024 * 1024) {
        return `${(bytes / 1024).toFixed(1)} KB`;
    }
    if (bytes < 1024 * 1024 * 1024) {
        return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
    }
    return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} GB`;
}

export function formatSize(kind: string, size: number | null | undefined): string {
    if (kind === FILE_KIND.PARENT) {
        return '';
    }
    if (kind === FILE_KIND.DIRECTORY || isNil(size)) {
        return '--';
    }
    return formatByteSize(size);
}

export function formatTransferSpeed(bytesPerSecond: number): string {
    return `${formatByteSize(bytesPerSecond)}/s`;
}

export function formatDate(kind: string, mtime: number | null | undefined): string {
    if (kind === FILE_KIND.PARENT) {
        return '';
    }
    if (isNil(mtime)) {
        return '--';
    }
    return new Date(mtime * 1000).toLocaleString();
}

export function formatKind(kind: string): string {
    if (kind === FILE_KIND.PARENT) {
        return '';
    }
    if (kind === FILE_KIND.DIRECTORY) {
        return 'folder';
    }
    if (kind === FILE_KIND.SYMLINK) {
        return 'symlink';
    }
    if (kind === FILE_KIND.FILE) {
        return 'file';
    }
    return 'other';
}

export function normalizeOsPathToSlash(path: string): string {
    return path.replace(/\\/g, '/');
}

export interface OsPathDirName {
    dir: string;
    name: string;
}

export function splitOsPathToDirAndName(path: string): OsPathDirName {
    const normalized = normalizeOsPathToSlash(path);
    const lastSlash = normalized.lastIndexOf('/');
    if (lastSlash >= 0) {
        return {dir: normalized.substring(0, lastSlash), name: normalized.substring(lastSlash + 1)};
    }
    return {dir: normalized, name: normalized};
}

export function groupPathsByParent(paths: string[]): Map<string, string[]> {
    const pathsByParent = new Map<string, string[]>();
    for (const p of paths) {
        const {dir, name} = splitOsPathToDirAndName(p);
        if (!name) {
            continue;
        }
        const existing = pathsByParent.get(dir);
        if (existing) {
            existing.push(name);
        } else {
            pathsByParent.set(dir, [name]);
        }
    }
    return pathsByParent;
}

export interface CssPoint {
    x: number;
    y: number;
}

export function physicalPositionToCssPoint(position: PhysicalPosition, dpr: number): CssPoint {
    return {x: position.x / dpr, y: position.y / dpr};
}

export function resolveWebviewDragDevicePixelRatio(): number {
    const dpr = window.devicePixelRatio;
    if (dpr) {
        return dpr;
    }
    return 1;
}

export function isPointInsideElement(element: HTMLElement | null, x: number, y: number): boolean {
    if (!element) {
        return false;
    }
    const rect = element.getBoundingClientRect();
    return x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom;
}

export function buildOsDropTransferPayloads(
    sessionId: string,
    targetPath: string,
    sourceClientTransferId: string,
    targetClientTransferId: string,
    pathsByParent: Map<string, string[]>,
): SftpTransferEvent[] {
    const payloads: SftpTransferEvent[] = [];
    for (const [sourcePath, names] of pathsByParent) {
        payloads.push({data: {names, sourcePath, targetPath, sideSessionId: sessionId, sourceClientTransferId, targetClientTransferId}});
    }
    return payloads;
}

export function splitSftpPathToSegments(path: string): string[] {
    if (!path) {
        return [];
    }
    if (path.startsWith('/')) {
        return ['/', ...path.split('/').filter(Boolean)];
    }
    return path.split('/').filter(Boolean);
}

export interface SftpBreadcrumbSegment {
    label: string;
    path: string;
    isEllipsis: boolean;
}

function buildSftpSegmentPath(segments: string[], endIndex: number): string {
    if (segments[0] === '/') {
        if (endIndex === 0) {
            return '/';
        }
        return '/' + segments.slice(1, endIndex + 1).join('/');
    }
    if (endIndex === 0) {
        return `${segments[0]}/`;
    }
    return segments.slice(0, endIndex + 1).join('/');
}

function toSftpBreadcrumbSegment(segments: string[], index: number): SftpBreadcrumbSegment {
    return {label: segments[index], path: buildSftpSegmentPath(segments, index), isEllipsis: false};
}

const SFTP_BREADCRUMB_TAIL_COUNT: Record<number, number> = {1: 3, 2: 2, 3: 1};

export function resolveSftpBreadcrumbSegments(segments: string[], collapseLevel: number): SftpBreadcrumbSegment[] {
    if (segments.length <= 4 || collapseLevel <= 0) {
        return segments.map((_, index) => toSftpBreadcrumbSegment(segments, index));
    }

    const headCount = 1;
    const tailCount = SFTP_BREADCRUMB_TAIL_COUNT[collapseLevel] ?? 1;
    if (segments.length <= headCount + tailCount) {
        return segments.map((_, index) => toSftpBreadcrumbSegment(segments, index));
    }

    const ellipsisIndex = segments.length - tailCount - 1;
    const ellipsisSegment: SftpBreadcrumbSegment = {label: '...', path: buildSftpSegmentPath(segments, ellipsisIndex), isEllipsis: true};
    const headSegments = Array.from({length: headCount}, (_, index) => toSftpBreadcrumbSegment(segments, index));
    const tailSegments = Array.from({length: tailCount}, (_, offset) => toSftpBreadcrumbSegment(segments, segments.length - tailCount + offset));

    return [...headSegments, ellipsisSegment, ...tailSegments];
}

export function filterSftpBreadcrumbToRoot(items: SftpBreadcrumbSegment[], rootPath: string): SftpBreadcrumbSegment[] {
    const root = rootPath.replace(/\/+$/, '');
    return items.filter(item => {
        const path = item.path.replace(/\/+$/, '');
        return path === root || path.startsWith(`${root}/`);
    });
}

export function getFileIconByExtension(filename: string): React.ElementType {
    if (!filename) {
        return InsertDriveFile;
    }
    const baseName = filename.split(/[\\/]/).pop();
    if (!baseName) {
        return InsertDriveFile;
    }
    const dotIndex = baseName.lastIndexOf('.');
    if (dotIndex <= 0) {
        return InsertDriveFile;
    }
    const extRaw = baseName.slice(dotIndex + 1);
    if (!extRaw) {
        return InsertDriveFile;
    }
    const ext = extRaw.toLowerCase();
    if (['zip', 'rar', '7z', 'tar', 'gz', 'bz2', 'xz', 'tgz'].includes(ext)) {
        return FolderZip;
    }
    if (['jpg', 'jpeg', 'png', 'gif', 'bmp', 'svg', 'webp', 'ico'].includes(ext)) {
        return Image;
    }
    if (['mp4', 'avi', 'mkv', 'mov', 'wmv', 'flv', 'webm', 'm4v'].includes(ext)) {
        return VideoFile;
    }
    if (['mp3', 'wav', 'flac', 'aac', 'ogg', 'm4a', 'wma'].includes(ext)) {
        return AudioFile;
    }
    if (['doc', 'docx', 'pdf', 'txt', 'rtf', 'odt'].includes(ext)) {
        return Description;
    }
    if (['xls', 'xlsx', 'csv', 'ods'].includes(ext)) {
        return TableChart;
    }
    if (['ppt', 'pptx', 'odp'].includes(ext)) {
        return Slideshow;
    }
    if (['js', 'jsx', 'ts', 'tsx', 'java', 'py', 'c', 'cpp', 'h', 'hpp', 'cs', 'go', 'rs', 'php', 'rb', 'swift', 'kt'].includes(ext)) {
        return Code;
    }
    if (['html', 'htm', 'xml', 'xhtml'].includes(ext)) {
        return Html;
    }
    if (['css', 'scss', 'sass', 'less'].includes(ext)) {
        return Css;
    }
    if (['json', 'yaml', 'yml', 'toml', 'ini', 'conf', 'config'].includes(ext)) {
        return DataObject;
    }
    if (['sql', 'db', 'sqlite', 'mdb'].includes(ext)) {
        return Storage;
    }
    if (['exe', 'msi', 'app', 'dmg', 'deb', 'rpm'].includes(ext)) {
        return Settings;
    }
    if (['ttf', 'otf', 'woff', 'woff2', 'eot'].includes(ext)) {
        return FontDownload;
    }
    if (['md', 'markdown'].includes(ext)) {
        return Article;
    }
    return InsertDriveFile;
}

export interface SftpSearch {
    hasMatch: boolean;
    indices: number[];
}

export function matchSftpEntryName(name: string, query: string): boolean {
    return name.toLowerCase().includes(query.toLowerCase());
}

export function searchSftpEntries(entries: FileEntry[], query: string | null): SftpSearch {
    if (!query) {
        return {hasMatch: true, indices: []};
    }
    const indices: number[] = [];
    for (let index = 0; index < entries.length; index++) {
        const entry = entries[index];
        if (entry.kind === FILE_KIND.PARENT) {
            continue;
        }
        if (matchSftpEntryName(entry.name, query)) {
            indices.push(index);
        }
    }
    return {hasMatch: indices.length > 0, indices};
}

export function filterSftpEntries(entries: FileEntry[], query: string | null): FileEntry[] {
    if (!query) {
        return entries;
    }
    return entries.filter(entry => {
        if (entry.kind === FILE_KIND.PARENT) {
            return true;
        }
        return matchSftpEntryName(entry.name, query);
    });
}

export function buildEntryNameSegments(name: string, query: string): Array<{ text: string; highlight: boolean }> {
    const segments: Array<{ text: string; highlight: boolean }> = [];
    const lowerName = name.toLowerCase();
    const lowerQuery = query.toLowerCase();
    let pos = 0;
    while (pos < name.length) {
        const idx = lowerName.indexOf(lowerQuery, pos);
        if (idx === -1) {
            segments.push({text: name.slice(pos), highlight: false});
            break;
        }
        if (idx > pos) {
            segments.push({text: name.slice(pos, idx), highlight: false});
        }
        segments.push({text: name.slice(idx, idx + query.length), highlight: true});
        pos = idx + query.length;
    }
    return segments;
}