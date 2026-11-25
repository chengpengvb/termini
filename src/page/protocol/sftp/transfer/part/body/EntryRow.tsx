import {memo} from 'react';
import type {MouseEvent} from 'react';
import {Box, Stack} from '@mui/material';
import Reply from '@mui/icons-material/Reply';
import Folder from '@mui/icons-material/Folder';
import InsertDriveFile from '@mui/icons-material/InsertDriveFile';
import {FILE_KIND, FileEntry} from '@/common/model/dto/FileEntry.ts';
import {usePlatformMobile} from '@/common/context/usePlatformMobile';
import {useLongPressMenu} from '@/page/common/hooks/useLongPressMenu.ts';
import {buildEntryNameSegments, formatDate, formatKind, formatPermissions, formatSize} from '@/page/common/util/sftpUtil.ts';

export interface EntryRowProps {
    entry: FileEntry;
    selected: boolean;
    selectionMode: boolean;
    searchQuery: string | null;
    onEntryDoubleClick: (entry: FileEntry) => void;
    onEntryClick: (name: string, multiSelect: boolean) => void;
    onEntryContextMenu: (name: string, event: MouseEvent<HTMLDivElement>) => void;
}

export const EntryRow = memo(function EntryRow({entry, selected, selectionMode, searchQuery, onEntryClick, onEntryDoubleClick, onEntryContextMenu}: EntryRowProps) {
    const mobileViewport = usePlatformMobile();
    const longPress = useLongPressMenu((clientX, clientY) => {
        if (entry.kind === FILE_KIND.PARENT) {
            return;
        }
        const event = {
            preventDefault: () => undefined,
            stopPropagation: () => undefined,
            clientX,
            clientY,
        } as MouseEvent<HTMLDivElement>;
        onEntryContextMenu(entry.name, event);
    });

    const resolveEntryIcon = () => {
        if (entry.kind === FILE_KIND.DIRECTORY || entry.kind === FILE_KIND.PARENT) {
            return <Folder className="tm-sftp-entry-icon-svg tm-sftp-entry-icon-accent"/>;
        }
        if (entry.kind === FILE_KIND.SYMLINK) {
            return <Reply className="tm-sftp-entry-icon-svg tm-sftp-entry-icon-accent tm-sftp-entry-icon-flip"/>;
        }
        return <InsertDriveFile className="tm-sftp-entry-icon-svg tm-sftp-entry-icon-file"/>;
    };

    const handleRowClick = (event: MouseEvent<HTMLDivElement>): void => {
        if (longPress.consumeIfFired()) {
            event.preventDefault();
            event.stopPropagation();
            return;
        }
        if (mobileViewport && selectionMode) {
            onEntryClick(entry.name, true);
            return;
        }
        if (mobileViewport) {
            if (entry.kind === FILE_KIND.PARENT) {
                onEntryDoubleClick(entry);
                return;
            }
            if (selected) {
                onEntryDoubleClick(entry);
                return;
            }
            onEntryClick(entry.name, false);
            return;
        }
        onEntryClick(entry.name, event.ctrlKey || event.metaKey);
    };

    const handleRowDoubleClick = (): void => {
        onEntryDoubleClick(entry);
    };

    const handleRowContextMenu = (event: MouseEvent<HTMLDivElement>): void => {
        event.preventDefault();
        event.stopPropagation();
        if (entry.kind === FILE_KIND.PARENT) {
            return;
        }
        onEntryContextMenu(entry.name, event);
    };

    const resolveRowClassName = (): string => {
        const classNames = ['tm-sftp-entry-row'];
        if (entry.hidden) {
            classNames.push('tm-sftp-entry-hidden');
        }
        if (selected) {
            classNames.push('tm-sftp-entry-row-selected');
        }
        return classNames.join(' ');
    };

    const resolveEntryName = () => {
        if (!searchQuery || entry.kind === FILE_KIND.PARENT) {
            return <span className="tm-sftp-entry-name">{entry.name}</span>;
        }
        return (
            <span className="tm-sftp-entry-name">
                {buildEntryNameSegments(entry.name, searchQuery).map((seg, i) => seg.highlight
                    ? <span key={i} className="tm-sftp-entry-name-match">{seg.text}</span>
                    : <span key={i}>{seg.text}</span>
                )}
            </span>
        );
    };

    return (
        <Box
            data-entry-row="1"
            className={resolveRowClassName()}
            onClick={handleRowClick}
            onDoubleClick={handleRowDoubleClick}
            onContextMenu={handleRowContextMenu}
            onPointerDown={longPress.handlePointerDown}
            onPointerMove={longPress.handlePointerMove}
            onPointerUp={longPress.handlePointerEnd}
            onPointerCancel={longPress.handlePointerEnd}
            onPointerLeave={longPress.handlePointerLeave}
        >
            <Box sx={{px: 2, minWidth: 0}}>
                <Stack direction="row" alignItems="center" spacing={1} className="tm-sftp-entry-name-cell">
                    <Stack alignItems="center" justifyContent="center" className="tm-sftp-entry-icon">
                        {resolveEntryIcon()}
                    </Stack>
                    <Stack className="tm-sftp-entry-name-group">
                        {resolveEntryName()}
                        <span className="tm-sftp-entry-perms">{formatPermissions(entry.kind, entry.permissions)}</span>
                    </Stack>
                </Stack>
            </Box>
            <Box sx={{px: 2, minWidth: 0, fontSize: 'var(--tm-sftp-entry-font-size)', color: 'var(--tm-text-secondary)', whiteSpace: 'nowrap', overflow: 'clip', textOverflow: 'ellipsis'}}>
                {formatDate(entry.kind, entry.mtime)}
            </Box>
            <Box sx={{px: 2, minWidth: 0, fontSize: 'var(--tm-sftp-entry-font-size)', color: 'var(--tm-text-secondary)', whiteSpace: 'nowrap', overflow: 'clip', textOverflow: 'ellipsis'}}>
                {formatSize(entry.kind, entry.size)}
            </Box>
            <Box sx={{px: 2, minWidth: 0, fontSize: 'var(--tm-sftp-entry-font-size)', color: 'var(--tm-text-secondary)', whiteSpace: 'nowrap', overflow: 'clip', textOverflow: 'ellipsis'}}>
                {formatKind(entry.kind)}
            </Box>
        </Box>
    );
});
