import { Box } from '@mui/material';
import type { RefObject } from 'react';
import Search from '@mui/icons-material/Search';
import { FileEntry } from "@/common/model/dto/FileEntry.ts";
import type { KeyboardEvent as ReactKeyboardEvent } from 'react';
import { searchSftpEntries } from '@/page/common/util/sftpUtil.ts';
import { useBackDismiss } from '@/page/common/hooks/useBackDismiss.ts';
import { BodyEntriesHandle } from '@/page/common/dto/sftp/BodyEntriesHandle.ts';
import { forwardRef, startTransition, useCallback, useEffect, useImperativeHandle, useMemo, useRef, useState } from 'react';

export interface BodySearchHandle {
    openSearch(): void;
}

export interface BodySearchProps {
    entries: FileEntry[];
    rootRef: RefObject<HTMLDivElement | null>;
    entriesHandleRef: RefObject<BodyEntriesHandle | null>;
}

export const BodySearch = forwardRef<BodySearchHandle, BodySearchProps>(function BodySearch(props, ref) {
    const searchCursorRef = useRef<number>(-1);
    const [searchOpen, setSearchOpen] = useState(false);
    const searchInputRef = useRef<HTMLInputElement | null>(null);
    const [searchQuery, setSearchQuery] = useState<string | null>(null);
    const { entries, rootRef, entriesHandleRef } = props;

    const sftpSearch = useMemo(() => {
        return searchSftpEntries(entries, searchQuery);
    }, [entries, searchQuery]);

    function publishSearchQuery(query: string | null): void {
        startTransition(() => {
            entriesHandleRef.current?.setSearchQuery(query);
        });
    }

    const closeSearch = useCallback((): void => {
        setSearchOpen(false);
        setSearchQuery(null);
        startTransition(() => {
            entriesHandleRef.current?.setSearchQuery(null);
        });
    }, [entriesHandleRef]);

    function isSearchTriggerActive(): boolean {
        return document.activeElement === rootRef.current;
    }

    function handleSearchNavigateNext(): void {
        if (!sftpSearch.hasMatch || sftpSearch.indices.length === 0) {
            return;
        }
        const nextCursor = (searchCursorRef.current + 1) % sftpSearch.indices.length;
        searchCursorRef.current = nextCursor;
        entriesHandleRef.current?.jumpToIndex(sftpSearch.indices[nextCursor]);
    }

    function handleSearchInputKeyDown(event: ReactKeyboardEvent<HTMLInputElement>): void {
        if (event.key === 'Escape') {
            event.stopPropagation();
            closeSearch();
            return;
        }
        if (event.key === 'Enter') {
            event.preventDefault();
            handleSearchNavigateNext();
        }
        if (event.key === 'Backspace') {
            if (searchQuery) {
                return;
            }
            event.preventDefault();
            closeSearch();
        }
    }

    function handleSearchQueryChange(value: string): void {
        setSearchQuery(value);
        if (!value) {
            publishSearchQuery(null);
            return;
        }
        publishSearchQuery(value);
    }

    function openSearch(): void {
        setSearchOpen(true);
        if (searchQuery === null) {
            setSearchQuery('');
        }
    }

    useImperativeHandle(ref, () => {
        return {openSearch};
    });

    useBackDismiss(searchOpen, closeSearch, 'sftp-body-search');

    useEffect(() => {
        searchCursorRef.current = -1;
    }, [searchQuery]);

    useEffect(() => {
        if (!searchOpen) {
            return;
        }
        searchInputRef.current?.focus({preventScroll: true});
    }, [searchOpen]);

    useEffect(() => {
        closeSearch();
    }, [entries, closeSearch]);

    useEffect(() => {
        function handleGlobalKeyDown(event: KeyboardEvent): void {
            if (!isSearchTriggerActive()) {
                return;
            }
            if (event.key === 'Escape') {
                closeSearch();
                return;
            }
            if (searchOpen) {
                return;
            }
            if (!event.ctrlKey && !event.metaKey && !event.altKey && event.key.length === 1) {
                event.preventDefault();
                setSearchOpen(true);
                setSearchQuery(event.key);
                publishSearchQuery(event.key);
            }
        }
        window.addEventListener('keydown', handleGlobalKeyDown);
        return () => window.removeEventListener('keydown', handleGlobalKeyDown);
    }, [closeSearch, entriesHandleRef, rootRef, searchOpen]);

    if (!searchOpen) {
        return null;
    }

    return (
        <Box className="tm-sftp-search-overlay">
            <Search className="tm-sftp-search-icon" />
            <input ref={searchInputRef} className={sftpSearch.hasMatch ? 'tm-sftp-search-input' : 'tm-sftp-search-input tm-sftp-search-input-no-match'} value={searchQuery!} onChange={e => handleSearchQueryChange(e.target.value)} onKeyDown={handleSearchInputKeyDown} />
        </Box>
    );
});
