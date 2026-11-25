import {FILE_KIND} from '@/common/model/dto/FileEntry.ts';
import type {BodyMenuContext} from '@/page/common/dto/sftp/BodyMenu.ts';

export const SFTP_PAGE_PANEL = {
    LEFT: 1,
    RIGHT: 2,
} as const;

export type SftpPagePanel = typeof SFTP_PAGE_PANEL[keyof typeof SFTP_PAGE_PANEL];

export const SFTP_PAGE_DRAG_THRESHOLD = 4;

export const SFTP_HEAD_ACTION = {
    COPY_TO_TARGET: 0,
    RENAME: 1,
    DELETE: 2,
    REFRESH: 3,
    NEW_FOLDER: 4,
    TOGGLE_HIDDEN: 5,
    CHMOD: 6,
    SELECT_ALL: 7,
    CLOSE: 8,
    NEW_FILE: 9,
    OPEN: 10,
    OPEN_WITH: 11,
    COPY_PATH: 12,
    SELECTION_MODE: 13,
    EXIT_SELECTION_MODE: 14,
    FILE_COPY: 15,
    FILE_PASTE: 16,
    SORT_BY_DATE: 17,
    IMPORT_FROM_FILES: 18,
} as const;

export const BODY_MENU_TARGET = {
    BLANK: 0,
    ENTRY_ROW: 1,
} as const;

export interface SftpHeadActionItem {
    code: number;
    labelI18n: string;
    danger: boolean;
    iosOnly?: boolean;
    mobileOnly?: boolean;
    visible: (ctx: BodyMenuContext) => boolean;
}

function alwaysVisible(): boolean {
    return true;
}

function visibleWhenSelected(ctx: BodyMenuContext): boolean {
    return ctx.selectedNames.length > 0;
}

function visibleOnEntrySingle(ctx: BodyMenuContext): boolean {
    if (ctx.target !== BODY_MENU_TARGET.ENTRY_ROW) {
        return false;
    }
    return ctx.selectedNames.length === 1;
}

function visibleOnSingleFile(ctx: BodyMenuContext): boolean {
    if (ctx.selectedEntries.length !== 1) {
        return false;
    }
    return ctx.selectedEntries[0].kind === FILE_KIND.FILE;
}

function visibleEnterSelectionMode(ctx: BodyMenuContext): boolean {
    return !ctx.selectionMode;
}

function visibleExitSelectionMode(ctx: BodyMenuContext): boolean {
    return ctx.selectionMode;
}

export const SFTP_HEAD_ACTION_ITEMS: SftpHeadActionItem[] = [
    {code: SFTP_HEAD_ACTION.OPEN, labelI18n: 'protocol.sftp.action.open', danger: false, visible: visibleOnEntrySingle},
    {code: SFTP_HEAD_ACTION.OPEN_WITH, labelI18n: 'protocol.sftp.action.openWith', danger: false, visible: visibleOnSingleFile},
    {code: SFTP_HEAD_ACTION.COPY_TO_TARGET, labelI18n: 'protocol.sftp.action.copyToTarget', danger: false, visible: visibleWhenSelected},
    {code: SFTP_HEAD_ACTION.FILE_COPY, labelI18n: 'protocol.sftp.action.copy', danger: false, mobileOnly: true, visible: visibleWhenSelected},
    {code: SFTP_HEAD_ACTION.FILE_PASTE, labelI18n: 'protocol.sftp.action.paste', danger: false, mobileOnly: true, visible: alwaysVisible},
    {code: SFTP_HEAD_ACTION.SELECTION_MODE, labelI18n: 'protocol.sftp.action.selectionMode', danger: false, mobileOnly: true, visible: visibleEnterSelectionMode},
    {code: SFTP_HEAD_ACTION.EXIT_SELECTION_MODE, labelI18n: 'protocol.sftp.action.exitSelectionMode', danger: false, mobileOnly: true, visible: visibleExitSelectionMode},
    {code: SFTP_HEAD_ACTION.SELECT_ALL, labelI18n: 'protocol.sftp.action.selectAll', danger: false, visible: alwaysVisible},
    {code: SFTP_HEAD_ACTION.SORT_BY_DATE, labelI18n: 'protocol.sftp.action.sortByDate', danger: false, mobileOnly: true, visible: alwaysVisible},
    {code: SFTP_HEAD_ACTION.REFRESH, labelI18n: 'protocol.sftp.action.refresh', danger: false, visible: alwaysVisible},
    {code: SFTP_HEAD_ACTION.RENAME, labelI18n: 'protocol.sftp.action.rename', danger: false, visible: visibleOnEntrySingle},
    {code: SFTP_HEAD_ACTION.NEW_FOLDER, labelI18n: 'protocol.sftp.action.newFolder', danger: false, visible: alwaysVisible},
    {code: SFTP_HEAD_ACTION.NEW_FILE, labelI18n: 'protocol.sftp.action.newFile', danger: false, visible: alwaysVisible},
    {code: SFTP_HEAD_ACTION.IMPORT_FROM_FILES, labelI18n: 'protocol.sftp.action.importFromFiles', danger: false, iosOnly: true, visible: alwaysVisible},
    {code: SFTP_HEAD_ACTION.DELETE, labelI18n: 'protocol.sftp.action.delete', danger: true, visible: visibleWhenSelected},
    {code: SFTP_HEAD_ACTION.TOGGLE_HIDDEN, labelI18n: 'protocol.sftp.action.toggleHidden', danger: false, visible: alwaysVisible},
    {code: SFTP_HEAD_ACTION.COPY_PATH, labelI18n: 'protocol.sftp.action.copyPath', danger: false, visible: visibleWhenSelected},
    {code: SFTP_HEAD_ACTION.CHMOD, labelI18n: 'protocol.sftp.action.chmod', danger: false, visible: visibleOnEntrySingle},
];
