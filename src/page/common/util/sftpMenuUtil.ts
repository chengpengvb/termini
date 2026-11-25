import type {RefObject} from 'react';
import {open} from '@tauri-apps/plugin-dialog';
import {translate} from '@/common/i18n/i18n.ts';
import type {SftpService} from '@/service/SftpService.ts';
import {FILE_KIND, FileEntry} from '@/common/model/dto/FileEntry.ts';
import type {SftpHandle} from '@/page/common/dto/sftp/SftpHandle.ts';
import type {SystemUtilService} from '@/service/SystemUtilService.ts';
import {RespSftpFileCopy, RespSftpFilePaste} from '@/common/model/resp/RespSftpTransfer.ts';
import {SftpChmodEvent, SftpCreateEvent, SftpDeleteEvent, SftpRenameEvent, SftpTransferEvent} from '@/common/model/req/event/WsSftp.ts';

export function resolveMenuNameError(entries: FileEntry[], name: string, originalName: string | null, slashText: string, existsText: string): string | null {
    const trimmed = name.trim();
    if (trimmed === '') {
        return null;
    }
    if (trimmed.includes('/')) {
        return slashText;
    }
    const conflict = entries.some(entry => entry.kind !== FILE_KIND.PARENT && entry.name === trimmed && entry.name !== originalName);
    if (conflict) {
        return existsText;
    }
    return null;
}

export function isMenuNameValid(entries: FileEntry[], name: string, originalName: string | null): boolean {
    const trimmed = name.trim();
    if (trimmed === '') {
        return false;
    }
    if (originalName !== null && trimmed === originalName) {
        return false;
    }
    if (trimmed.includes('/')) {
        return false;
    }
    return !entries.some(entry => entry.kind !== FILE_KIND.PARENT && entry.name === trimmed && entry.name !== originalName);
}

function reportNotReady(sftpService: SftpService, sessionId: string, reason: string): void {
    console.warn(`[sftp] ${reason}`);
    sftpService.notifyLocalError(sessionId, translate('protocol.sftp.notReady'));
}

export function doOpen(onOpen: (entry: FileEntry, openWith: boolean) => void, files: FileEntry[]): void {
    if (files.length != 1) {
        return;
    }
    onOpen(files[0], false);
}

export function doOpenWith(onOpen: (entry: FileEntry, openWith: boolean) => void, files: FileEntry[]): void {
    if (files.length != 1) {
        return;
    }
    onOpen(files[0], true);
}

export function doTransfer(sftpService: SftpService, sessionId: string, sideRef: RefObject<SftpHandle | null> | undefined, names: string[]): void {
    if (!names || names.length === 0) {
        return;
    }
    if (!sideRef || !sideRef.current) {
        reportNotReady(sftpService, sessionId, 'transfer skipped: peer pane is not mounted');
        return;
    }
    const targetPath = sideRef.current.getCurrentPath();
    const targetClientTransferId = sideRef.current.getClientTransferId();
    if (!targetPath || !targetClientTransferId) {
        reportNotReady(sftpService, sessionId, 'transfer skipped: peer pane has no loaded directory');
        return;
    }
    const sourcePath = sftpService.getCurrentPath(sessionId);
    const sourceClientTransferId = sftpService.getClientTransferId(sessionId);
    if (!sourcePath || !sourceClientTransferId) {
        reportNotReady(sftpService, sessionId, 'transfer skipped: source pane has no loaded directory');
        return;
    }
    const payload: SftpTransferEvent = {
        data: {
            names: names,
            sourcePath: sourcePath,
            targetPath: targetPath,
            sideSessionId: sideRef.current.getSessionId(),
            sourceClientTransferId: sourceClientTransferId,
            targetClientTransferId: targetClientTransferId,
        },
    };
    sftpService.handleTransfer(sessionId, payload);
}

export function doFileCopy(sftpService: SftpService, sessionId: string, entries: FileEntry[]): void {
    if (!entries || entries.length === 0) {
        return;
    }
    const sourcePath = sftpService.getCurrentPath(sessionId);
    const sourceClientTransferId = sftpService.getClientTransferId(sessionId);
    if (!sourcePath || !sourceClientTransferId) {
        return;
    }
    const payload: RespSftpFileCopy = {
        sourcePath,
        sourceClientTransferId,
        sideSessionId: sessionId,
        names: entries.map(entry => entry.name),
    };
    sftpService.setFileCopy(payload).catch(console.error);
}

export function doFilePaste(sftpService: SftpService, sessionId: string): void {
    const targetPath = sftpService.getCurrentPath(sessionId);
    const targetClientTransferId = sftpService.getClientTransferId(sessionId);
    if (!targetPath || !targetClientTransferId) {
        return;
    }
    const payload: RespSftpFilePaste = {
        targetPath,
        targetClientTransferId,
        sideSessionId: sessionId,
    };
    sftpService.setFilePaste(payload).catch(console.error);
}

export function doDelete(sftpService: SftpService, sessionId: string, entries: FileEntry[]): void {
    if (!entries || entries.length === 0) {
        return;
    }
    const clientTransferId = sftpService.getClientTransferId(sessionId);
    const currentPath = sftpService.getCurrentPath(sessionId);
    if (!clientTransferId || !currentPath) {
        reportNotReady(sftpService, sessionId, 'delete skipped: session has no loaded directory');
        return;
    }
    const names = entries.map(entry => entry.name);
    const payload: SftpDeleteEvent = {
        data: {
            names: names,
            path: currentPath,
            clientTransferId: clientTransferId,
        },
    };
    sftpService.handleDelete(sessionId, payload);
}

export function doRefresh(sftpService: SftpService, sessionId: string): void {
    const currentPath = sftpService.getCurrentPath(sessionId);
    if (!currentPath) {
        return;
    }
    sftpService.handleListDir(sessionId, currentPath);
}

export async function doCopyPath(systemUtilService: SystemUtilService, entries: FileEntry[]): Promise<void> {
    if (!entries || entries.length === 0) {
        return;
    }
    const text = entries
        .map(entry => entry.path)
        .filter(Boolean)
        .join('\n');
    if (!text) {
        return;
    }
    await systemUtilService.writeClipboardText(text);
}

export async function doImportFromFiles(sftpService: SftpService, systemUtilService: SystemUtilService, sessionId: string): Promise<void> {
    const dest = sftpService.getCurrentPath(sessionId);
    if (!dest) {
        reportNotReady(sftpService, sessionId, 'import skipped: session has no loaded directory');
        return;
    }
    let selected: string | string[] | null;
    try {
        selected = await open({multiple: true, pickerMode: 'document', fileAccessMode: 'copy'});
    } catch (error) {
        sftpService.notifyLocalError(sessionId, String(error));
        return;
    }
    if (selected === null) {
        return;
    }
    const sources = Array.isArray(selected) ? selected : [selected];
    if (sources.length === 0) {
        return;
    }
    try {
        await systemUtilService.copyLocalInto(dest, sources);
    } catch (error) {
        sftpService.notifyLocalError(sessionId, String(error));
        return;
    }
    doRefresh(sftpService, sessionId);
}

export function submitRename(sftpService: SftpService, sessionId: string, entry: FileEntry, newName: string): void {
    const clientTransferId = sftpService.getClientTransferId(sessionId);
    if (!clientTransferId) {
        return;
    }
    const currentPath = sftpService.getCurrentPath(sessionId);
    if (!currentPath) {
        return;
    }
    const payload: SftpRenameEvent = {data: {path: currentPath, fromName: entry.name, toName: newName, clientTransferId: clientTransferId}};
    sftpService.handleRename(sessionId, payload);
}

export function submitCreate(sftpService: SftpService, sessionId: string, kind: typeof FILE_KIND.FILE | typeof FILE_KIND.DIRECTORY, name: string): void {
    const currentPath = sftpService.getCurrentPath(sessionId);
    const clientTransferId = sftpService.getClientTransferId(sessionId);
    if (!currentPath || !clientTransferId) {
        return;
    }
    const payload: SftpCreateEvent = {
        data: {
            name: name,
            path: currentPath,
            kind: kind,
            clientTransferId: clientTransferId,
        },
    };
    sftpService.handleCreate(sessionId, payload);
}

export function submitChmod(sftpService: SftpService, sessionId: string, path: string, entries: FileEntry[], mode: number): void {
    const clientTransferId = sftpService.getClientTransferId(sessionId);
    if (!clientTransferId) {
        return;
    }
    const names = entries.map(entry => entry.name);
    const payload: SftpChmodEvent = {data: {mode, path, names, clientTransferId}};
    sftpService.handleChmod(sessionId, payload);
}
