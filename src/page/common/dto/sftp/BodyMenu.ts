import type {MouseEvent, RefObject} from 'react';
import {FILE_KIND} from '@/common/model/dto/FileEntry.ts';
import type {FileEntry} from '@/common/model/dto/FileEntry.ts';
import type {SftpHandle} from '@/page/common/dto/sftp/SftpHandle.ts';

export interface RenameDialogState {
    entry: FileEntry;
    name: string;
}

export interface CreateDialogState {
    name: string;
    kind: typeof FILE_KIND.FILE | typeof FILE_KIND.DIRECTORY;
}

export interface DeleteDialogState {
    entries: FileEntry[];
}

export interface TransferDialogState {
    names: string[];
    targetPath: string;
}

export interface BodyMenuContext {
    nonce: number;
    target: number;
    selectionMode: boolean;
    selectedNames: string[];
    entryName: string | null;
    selectedEntries: FileEntry[];
    event: MouseEvent<HTMLDivElement>;
}

export interface ChmodDialogState {
    path: string;
    entries: FileEntry[];
}

export interface PermissionRow {
    label: string;
    read: number;
    write: number;
    execute: number;
}

export interface BodyMenuDialogsHandle {
    openRename(entries: FileEntry[]): void;
    openCreate(kind: typeof FILE_KIND.FILE | typeof FILE_KIND.DIRECTORY): void;
    openDelete(entries: FileEntry[]): void;
    openChmod(entries: FileEntry[]): void;
    requestTransfer(names: string[]): void;
}

export interface BodyMenuProps {
    protocol: number;
    sessionId: string;
    localPane: boolean;
    showHidden: boolean;
    entries: FileEntry[];
    selectionMode: boolean;
    onSelectAll: () => void;
    onSortByDate: () => void;
    onToggleHidden: () => void;
    onEnterSelectionMode: () => void;
    onExitSelectionMode: () => void;
    menuContext: BodyMenuContext | null;
    sideRef?: RefObject<SftpHandle | null>;
    resolveSelectedEntries: () => FileEntry[];
    focusRootRef: RefObject<HTMLElement | null>;
    dialogContainerRef?: RefObject<HTMLElement | null>;
    onOpen: (entry: FileEntry, openWith: boolean) => void;
}
