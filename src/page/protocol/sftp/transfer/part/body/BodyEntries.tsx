import { Box } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { RespSftpEntries } from '@/common/model/resp/RespSftp.ts';
import { SftpHandle } from '@/page/common/dto/sftp/SftpHandle.ts';
import { useSftpService } from '@/common/context/useAppHooks.tsx';
import { FILE_KIND, FileEntry } from '@/common/model/dto/FileEntry.ts';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { KeyboardArrowDown, KeyboardArrowUp } from '@mui/icons-material';
import type { BodyMenuContext } from '@/page/common/dto/sftp/BodyMenu.ts';
import { BODY_MENU_TARGET } from '@/page/common/dto/sftp/SftpHeadConst.ts';
import { BodyMenu } from '@/page/protocol/sftp/transfer/part/body/BodyMenu.tsx';
import { BodyEntriesVirtual } from '@/page/protocol/sftp/transfer/part/body/BodyEntriesVirtual.tsx';
import { BodySearch, BodySearchHandle } from '@/page/protocol/sftp/transfer/part/body/BodySearch.tsx';
import { BodyEntriesHandle, BodyEntriesVirtualHandle } from '@/page/common/dto/sftp/BodyEntriesHandle.ts';
import { filterSftpEntries, SFTP_ENTRY_SORT_COLUMN, SFTP_ENTRY_SORT_ORDER, sortSftpEntries } from '@/page/common/util/sftpUtil.ts';
import { Dispatch, MouseEvent, RefObject, SetStateAction, useEffect, useImperativeHandle, useLayoutEffect, useMemo, useRef, useState } from 'react';

export interface BodyEntriesProps {
  protocol: number;
  sessionId: string;
  showHidden: boolean;
  localPane: boolean;
  sftpEntries: RespSftpEntries;
  sideRef?: RefObject<SftpHandle | null>;
  setLoading: Dispatch<SetStateAction<boolean>>;
  setShowHidden: Dispatch<SetStateAction<boolean>>;
  dialogContainerRef?: RefObject<HTMLElement | null>;
  entriesHandleRef: RefObject<BodyEntriesHandle | null>;
  virtualRef: RefObject<BodyEntriesVirtualHandle | null>;
}

export function BodyEntries(props: BodyEntriesProps) {
  const { t } = useI18n();
  const sftpService = useSftpService();
  const mobileViewport = usePlatformMobile();
  const menuNonceRef = useRef<number>(0);
  const [, triggerSelectionRender] = useState(0);
  const rootRef = useRef<HTMLDivElement | null>(null);
  const selectedNamesRef = useRef<Set<string>>(new Set());
  const searchHandleRef = useRef<BodySearchHandle | null>(null);
  const [selectionMode, setSelectionMode] = useState(false);
  const [filterQuery, setFilterQuery] = useState<string | null>(null);
  const [menuContext, setMenuContext] = useState<BodyMenuContext | null>(null);
  const { sessionId, sftpEntries, setLoading, sideRef, virtualRef, showHidden, setShowHidden, dialogContainerRef, entriesHandleRef, localPane, protocol } = props;

  function resolveInitialSortColumn(): number {
    const saved = sftpService.getEntrySort(sessionId);
    if (!saved) {
      return SFTP_ENTRY_SORT_COLUMN.NAME;
    }
    return saved.column;
  }

  function resolveInitialSortOrder(): number {
    const saved = sftpService.getEntrySort(sessionId);
    if (!saved) {
      return SFTP_ENTRY_SORT_ORDER.ASC;
    }
    return saved.order;
  }

  const [sortOrder, setSortOrder] = useState<number>(resolveInitialSortOrder);
  const [sortColumn, setSortColumn] = useState<number>(resolveInitialSortColumn);

  const entries = sftpEntries.entries;
  const selectedNames = selectedNamesRef.current;

  const displayEntries = useMemo(() => {
    const visible = showHidden ? entries : entries.filter(entry => entry.kind === FILE_KIND.PARENT || !entry.hidden);
    const sorted = sortSftpEntries(visible, sortColumn, sortOrder);
    return filterSftpEntries(sorted, filterQuery);
  }, [entries, sortColumn, sortOrder, showHidden, filterQuery]);

  function resolveSelectedEntries(): FileEntry[] {
    const names = selectedNamesRef.current;
    return entries.filter(entry => {
      if (entry.kind === FILE_KIND.PARENT) {
        return false;
      }
      return names.has(entry.name);
    });
  }

  function handleEntryClick(name: string, multiSelect: boolean): void {
    const entry = entries.find(item => item.name === name);
    if (entry && entry.kind === FILE_KIND.PARENT) {
      return;
    }
    const selectedNames = selectedNamesRef.current;
    if (!multiSelect) {
      selectedNames.clear();
      selectedNames.add(name);
      triggerSelectionRender(value => value + 1);
      return;
    }
    if (selectedNames.has(name)) {
      selectedNames.delete(name);
      triggerSelectionRender(value => value + 1);
      return;
    }
    selectedNames.add(name);
    triggerSelectionRender(value => value + 1);
  }

  function handleDragSelect(names: string[], append: boolean): void {
    const selectedNames = selectedNamesRef.current;
    if (!append) {
      selectedNames.clear();
      for (const name of names) {
        selectedNames.add(name);
      }
      triggerSelectionRender(value => value + 1);
      return;
    }
    for (const name of names) {
      selectedNames.add(name);
    }
    triggerSelectionRender(value => value + 1);
  }

  function handleClearSelection(): void {
    const selectedNames = selectedNamesRef.current;
    if (selectedNames.size === 0) {
      return;
    }
    selectedNames.clear();
    triggerSelectionRender(value => value + 1);
  }

  function handleSelectAll(): void {
    const selectedNames = selectedNamesRef.current;
    selectedNames.clear();
    for (const entry of displayEntries) {
      if (entry.kind === FILE_KIND.PARENT) {
        continue;
      }
      selectedNames.add(entry.name);
    }
    if (mobileViewport) {
      setSelectionMode(true);
    }
    triggerSelectionRender(value => value + 1);
  }

  function handleEnterSelectionMode(): void {
    setSelectionMode(true);
  }

  function handleExitSelectionMode(): void {
    setSelectionMode(false);
  }

  function handleSortByDate(): void {
    handleSortColumnClick(SFTP_ENTRY_SORT_COLUMN.DATE);
  }

  function handleOpenEntry(fileEntry: FileEntry, openWith: boolean): void {
    if (fileEntry.kind === FILE_KIND.PARENT || fileEntry.kind === FILE_KIND.DIRECTORY || fileEntry.kind === FILE_KIND.SYMLINK) {
      setLoading(true);
      sftpService.handleListDir(sessionId, fileEntry.path);
      return;
    }
    if (fileEntry.kind !== FILE_KIND.FILE) {
      return;
    }
    const clientTransferId = sftpService.getClientTransferId(sessionId);
    if (!clientTransferId) {
      return;
    }
    sftpService.handleFileOpen(sessionId, {
      data: {
        path: fileEntry.path,
        openWith: openWith,
        clientTransferId: clientTransferId,
      },
    });
  }

  function handleEntryDoubleClick(fileEntry: FileEntry): void {
    handleOpenEntry(fileEntry, false);
  }

  function applySort(column: number, order: number): void {
    setSortColumn(column);
    setSortOrder(order);
    sftpService.saveEntrySort(sessionId, { column, order });
  }

  function handleSortColumnClick(column: number): void {
    if (sortColumn === column) {
      if (sortOrder === SFTP_ENTRY_SORT_ORDER.ASC) {
        applySort(column, SFTP_ENTRY_SORT_ORDER.DESC);
        return;
      }
      applySort(column, SFTP_ENTRY_SORT_ORDER.ASC);
      return;
    }
    applySort(column, SFTP_ENTRY_SORT_ORDER.ASC);
  }

  function resolveSortIcon(column: number) {
    if (sortColumn !== column) {
      return null;
    }
    if (sortOrder === SFTP_ENTRY_SORT_ORDER.DESC) {
      return <KeyboardArrowDown className="tm-sftp-entries-header-sort-icon" />;
    }
    return <KeyboardArrowUp className="tm-sftp-entries-header-sort-icon" />;
  }

  function resolveHeaderCellClassName(column: number): string {
    if (sortColumn === column) {
      return 'tm-sftp-entries-header-cell tm-sftp-entries-header-cell-active';
    }
    return 'tm-sftp-entries-header-cell';
  }

  function handleRootMouseDown(): void {
    rootRef.current?.focus({ preventScroll: true });
  }

  function getScrollTop(): number {
    return virtualRef.current?.getScrollTop() ?? 0;
  }

  function jumpToIndex(index: number): void {
    const entry = displayEntries[index];
    if (!entry) {
      return;
    }
    virtualRef.current?.scrollToIndex(index);
    selectedNamesRef.current.clear();
    selectedNamesRef.current.add(entry.name);
    triggerSelectionRender(value => value + 1);
  }

  function applyFilterQuery(query: string | null): void {
    if (!query) {
      setFilterQuery(null);
      return;
    }
    setFilterQuery(query);
  }

  function applySearchQuery(query: string | null): void {
    virtualRef.current?.setSearchQuery(query);
  }

  function buildMenuContext(event: MouseEvent<HTMLDivElement>, nonce: number): BodyMenuContext {
    const selectedNames = Array.from(selectedNamesRef.current);
    const selectedEntries = resolveSelectedEntries();
    const isSingleEntryTarget = selectedNames.length === 1 && selectedEntries.length === 1 && !selectionMode;
    const target = isSingleEntryTarget ? BODY_MENU_TARGET.ENTRY_ROW : BODY_MENU_TARGET.BLANK;
    const entryName = isSingleEntryTarget ? selectedEntries[0].name : null;
    return { nonce, target, entryName, selectedNames, selectedEntries, selectionMode, event };
  }

  function openActionsMenu(clientX: number, clientY: number): void {
    menuNonceRef.current += 1;
    const event = { clientX, clientY } as MouseEvent<HTMLDivElement>;
    setMenuContext(buildMenuContext(event, menuNonceRef.current));
  }

  function openSearch(): void {
    searchHandleRef.current?.openSearch();
  }

  function handleBlankContextMenu(event: MouseEvent<HTMLDivElement>): void {
    event.preventDefault();
    menuNonceRef.current += 1;
    setMenuContext({
      event,
      selectionMode,
      entryName: null,
      nonce: menuNonceRef.current,
      target: BODY_MENU_TARGET.BLANK,
      selectedEntries: resolveSelectedEntries(),
      selectedNames: Array.from(selectedNamesRef.current),
    });
  }

  function handleEntryContextMenu(name: string, event: MouseEvent<HTMLDivElement>): void {
    event.preventDefault();
    event.stopPropagation();
    const entry = entries.find(item => item.name === name);
    if (entry && entry.kind === FILE_KIND.PARENT) {
      return;
    }
    const selectedNames = selectedNamesRef.current;
    if (!selectedNames.has(name)) {
      if (!selectionMode) {
        selectedNames.clear();
      }
      selectedNames.add(name);
      triggerSelectionRender(value => value + 1);
    }
    menuNonceRef.current += 1;
    const nextSelected = Array.from(selectedNamesRef.current);
    const nextEntries = resolveSelectedEntries();
    const singleTarget = nextSelected.length === 1 && !selectionMode;
    setMenuContext({
      event,
      selectionMode,
      nonce: menuNonceRef.current,
      selectedNames: nextSelected,
      selectedEntries: nextEntries,
      entryName: singleTarget ? name : null,
      target: singleTarget ? BODY_MENU_TARGET.ENTRY_ROW : BODY_MENU_TARGET.BLANK,
    });
  }

  useImperativeHandle(entriesHandleRef, () => {
    const handle: BodyEntriesHandle = {
      jumpToIndex,
      getScrollTop,
      openSearch,
      openActionsMenu,
      setFilterQuery: applyFilterQuery,
      setSearchQuery: applySearchQuery,
    };
    return handle;
  });

  useLayoutEffect(() => {
    const currentPath = sftpService.getCurrentPath(sessionId);
    if (!currentPath) {
      return;
    }
    const savedScrollTop = sftpService.getPathScrollTop(sessionId, currentPath);
    virtualRef.current?.setScrollTop(savedScrollTop === null ? 0 : savedScrollTop);
  }, [displayEntries.length, entries, sessionId, sftpService, virtualRef]);

  useEffect(() => {
    selectedNamesRef.current.clear();
    setSelectionMode(false);
  }, [entries]);

  return (
    <Box
      ref={rootRef}
      tabIndex={-1}
      className={selectionMode ? 'tm-sftp-entries-root is-selection-mode' : 'tm-sftp-entries-root'}
      sx={{ display: 'flex', flexDirection: 'column', width: '100%', flex: 1, minHeight: 0, position: 'relative' }}
      onMouseDown={handleRootMouseDown}
      onContextMenu={handleBlankContextMenu}
    >
      <Box className="tm-sftp-entries-header">
        <BodySearch ref={searchHandleRef} entries={displayEntries} rootRef={rootRef} entriesHandleRef={entriesHandleRef} />
        <Box onClick={() => handleSortColumnClick(SFTP_ENTRY_SORT_COLUMN.NAME)} className={resolveHeaderCellClassName(SFTP_ENTRY_SORT_COLUMN.NAME)}>
          <span className="tm-sftp-entries-header-label">{t('common.name')}</span>
          {resolveSortIcon(SFTP_ENTRY_SORT_COLUMN.NAME)}
        </Box>
        <Box onClick={() => handleSortColumnClick(SFTP_ENTRY_SORT_COLUMN.DATE)} className={resolveHeaderCellClassName(SFTP_ENTRY_SORT_COLUMN.DATE)}>
          <span className="tm-sftp-entries-header-label">{t('common.modifiedDate')}</span>
          {resolveSortIcon(SFTP_ENTRY_SORT_COLUMN.DATE)}
        </Box>
        <Box onClick={() => handleSortColumnClick(SFTP_ENTRY_SORT_COLUMN.SIZE)} className={resolveHeaderCellClassName(SFTP_ENTRY_SORT_COLUMN.SIZE)}>
          <span className="tm-sftp-entries-header-label">{t('common.size')}</span>
          {resolveSortIcon(SFTP_ENTRY_SORT_COLUMN.SIZE)}
        </Box>
        <Box onClick={() => handleSortColumnClick(SFTP_ENTRY_SORT_COLUMN.KIND)} className={resolveHeaderCellClassName(SFTP_ENTRY_SORT_COLUMN.KIND)}>
          <span className="tm-sftp-entries-header-label">{t('common.type')}</span>
          {resolveSortIcon(SFTP_ENTRY_SORT_COLUMN.KIND)}
        </Box>
      </Box>

      <BodyEntriesVirtual
        ref={virtualRef}
        entries={displayEntries}
        selectedNames={selectedNames}
        selectionMode={selectionMode}
        onEntryClick={handleEntryClick}
        onEntryDoubleClick={handleEntryDoubleClick}
        onEntryContextMenu={handleEntryContextMenu}
        onBlankContextMenu={handleBlankContextMenu}
        onDragSelect={handleDragSelect}
        onClearSelection={handleClearSelection}
      />
      <BodyMenu
        sessionId={sessionId}
        entries={entries}
        menuContext={menuContext}
        focusRootRef={rootRef}
        resolveSelectedEntries={resolveSelectedEntries}
        sideRef={sideRef}
        showHidden={showHidden}
        selectionMode={selectionMode}
        dialogContainerRef={dialogContainerRef}
        onToggleHidden={() => setShowHidden(!showHidden)}
        onSelectAll={handleSelectAll}
        onEnterSelectionMode={handleEnterSelectionMode}
        onExitSelectionMode={handleExitSelectionMode}
        onSortByDate={handleSortByDate}
        onOpen={handleOpenEntry}
        localPane={localPane}
        protocol={protocol}
      />
    </Box>
  );
}
