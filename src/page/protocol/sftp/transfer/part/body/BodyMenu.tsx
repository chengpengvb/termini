import { Box } from '@mui/material';
import { PROTOCOL } from '@/common/const/Const.ts';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { useEffect, useRef, useState } from 'react';
import { FILE_KIND } from '@/common/model/dto/FileEntry.ts';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { useSftpService, useSystemUtilService } from '@/common/context/useAppHooks.tsx';
import { BodyMenuDialogs } from '@/page/protocol/sftp/transfer/part/body/BodyMenuDialogs.tsx';
import { BodyMenuList, resolveMenuActionLabel } from '@/page/protocol/sftp/transfer/part/body/BodyMenuList.tsx';
import type { BodyMenuContext, BodyMenuDialogsHandle, BodyMenuProps } from '@/page/common/dto/sftp/BodyMenu.ts';
import { BODY_MENU_TARGET, SFTP_HEAD_ACTION, SFTP_HEAD_ACTION_ITEMS } from '@/page/common/dto/sftp/SftpHeadConst.ts';
import { doCopyPath, doFileCopy, doFilePaste, doImportFromFiles, doOpen, doOpenWith, doRefresh } from '@/page/common/util/sftpMenuUtil.ts';

const SELECTION_BAR_CODES = [SFTP_HEAD_ACTION.COPY_TO_TARGET, SFTP_HEAD_ACTION.FILE_COPY, SFTP_HEAD_ACTION.FILE_PASTE, SFTP_HEAD_ACTION.DELETE, SFTP_HEAD_ACTION.EXIT_SELECTION_MODE];

function isEditableKeyboardTarget(target: EventTarget | null): boolean {
  return target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement || (target instanceof HTMLElement && target.isContentEditable);
}

function canHandleBodyShortcut(root: HTMLElement, event: KeyboardEvent): boolean {
  if (!root.contains(document.activeElement)) {
    return false;
  }
  return !isEditableKeyboardTarget(event.target);
}

export function BodyMenu(props: BodyMenuProps) {
  const { t, tf } = useI18n();
  const sftpService = useSftpService();
  const mobileViewport = usePlatformMobile();
  const [closed, setClosed] = useState(false);
  const systemUtilService = useSystemUtilService();
  const [iosPlatform, setIosPlatform] = useState(false);
  const lastMenuContextRef = useRef<BodyMenuContext | null>(null);
  const dialogsRef = useRef<BodyMenuDialogsHandle | null>(null);
  const { sessionId, entries, menuContext, sideRef, showHidden, selectionMode, localPane, onToggleHidden, onSelectAll, onEnterSelectionMode, onExitSelectionMode, onSortByDate, focusRootRef, resolveSelectedEntries, dialogContainerRef, onOpen, protocol } = props;
  const chmodEnabled = localPane || protocol !== PROTOCOL.FTP;

  function closeMenu(): void {
    setClosed(true);
  }

  function resolveLiveMenuContext(): BodyMenuContext {
    const selectedEntries = resolveSelectedEntries();
    return {
      nonce: 0,
      entryName: null,
      selectionMode: true,
      selectedEntries,
      target: BODY_MENU_TARGET.BLANK,
      selectedNames: selectedEntries.map(entry => entry.name),
      event: { clientX: 0, clientY: 0, preventDefault: () => undefined, stopPropagation: () => undefined } as BodyMenuContext['event'],
    };
  }

  function handleItemClick(code: number, actionContext: BodyMenuContext): void {
    setClosed(true);
    const dialogs = dialogsRef.current;
    if (!dialogs) {
      throw new Error('BodyMenuDialogs handle missing');
    }
    switch (code) {
      case SFTP_HEAD_ACTION.OPEN:
        doOpen(onOpen, actionContext.selectedEntries);
        break;
      case SFTP_HEAD_ACTION.OPEN_WITH:
        doOpenWith(onOpen, actionContext.selectedEntries);
        break;
      case SFTP_HEAD_ACTION.COPY_TO_TARGET:
        dialogs.requestTransfer(actionContext.selectedNames);
        break;
      case SFTP_HEAD_ACTION.FILE_COPY:
        doFileCopy(sftpService, sessionId, actionContext.selectedEntries);
        break;
      case SFTP_HEAD_ACTION.FILE_PASTE:
        doFilePaste(sftpService, sessionId);
        break;
      case SFTP_HEAD_ACTION.SELECTION_MODE:
        onEnterSelectionMode();
        break;
      case SFTP_HEAD_ACTION.EXIT_SELECTION_MODE:
        onExitSelectionMode();
        break;
      case SFTP_HEAD_ACTION.SORT_BY_DATE:
        onSortByDate();
        break;
      case SFTP_HEAD_ACTION.DELETE:
        dialogs.openDelete(actionContext.selectedEntries);
        break;
      case SFTP_HEAD_ACTION.REFRESH:
        doRefresh(sftpService, sessionId);
        break;
      case SFTP_HEAD_ACTION.RENAME:
        dialogs.openRename(actionContext.selectedEntries);
        break;
      case SFTP_HEAD_ACTION.NEW_FOLDER:
        dialogs.openCreate(FILE_KIND.DIRECTORY);
        break;
      case SFTP_HEAD_ACTION.NEW_FILE:
        dialogs.openCreate(FILE_KIND.FILE);
        break;
      case SFTP_HEAD_ACTION.IMPORT_FROM_FILES:
        void doImportFromFiles(sftpService, systemUtilService, sessionId);
        break;
      case SFTP_HEAD_ACTION.TOGGLE_HIDDEN:
        onToggleHidden();
        break;
      case SFTP_HEAD_ACTION.COPY_PATH:
        void doCopyPath(systemUtilService, actionContext.selectedEntries);
        break;
      case SFTP_HEAD_ACTION.CHMOD:
        dialogs.openChmod(actionContext.selectedEntries);
        break;
      case SFTP_HEAD_ACTION.SELECT_ALL:
        onSelectAll();
        break;
    }
  }

  function handleBodyShortcutKeyDown(event: KeyboardEvent): void {
    if (event.key === 'F2') {
      event.preventDefault();
      const dialogs = dialogsRef.current;
      if (!dialogs) {
        throw new Error('BodyMenuDialogs handle missing');
      }
      dialogs.openRename(resolveSelectedEntries());
      return;
    }
    if (!(event.ctrlKey || event.metaKey)) {
      return;
    }
    const key = event.key.toLowerCase();
    if (key === 'a') {
      event.preventDefault();
      onSelectAll();
      return;
    }
    if (key === 'c') {
      event.preventDefault();
      doFileCopy(sftpService, sessionId, resolveSelectedEntries());
      return;
    }
    if (key === 'v') {
      event.preventDefault();
      doFilePaste(sftpService, sessionId);
    }
  }

  useEffect(() => {
    systemUtilService.getPlatformInfo().then(info => {
      setIosPlatform(info.os === 'ios');
    });
  }, [systemUtilService]);

  useEffect(() => {
    const root = focusRootRef.current;
    if (!root) {
      return;
    }
    const onKeyDown = (event: KeyboardEvent): void => {
      if (!canHandleBodyShortcut(root, event)) {
        return;
      }
      handleBodyShortcutKeyDown(event);
    };
    root.addEventListener('keydown', onKeyDown);
    return () => {
      root.removeEventListener('keydown', onKeyDown);
    };
  }, [focusRootRef, resolveSelectedEntries, onSelectAll, sessionId]);

  const openNonceRef = useRef<number | null>(null);
  if (menuContext && menuContext.nonce !== openNonceRef.current) {
    openNonceRef.current = menuContext.nonce;
    if (closed) {
      setClosed(false);
    }
  }

  const showMenu = Boolean(menuContext && !closed);
  const showSelectionBar = mobileViewport && selectionMode;
  if (menuContext) {
    lastMenuContextRef.current = menuContext;
  }
  const renderMenuContext = menuContext ?? lastMenuContextRef.current;
  const selectionBarContext = showSelectionBar ? resolveLiveMenuContext() : null;
  const selectionBarItems = selectionBarContext
    ? SFTP_HEAD_ACTION_ITEMS.filter(item => {
        if (!SELECTION_BAR_CODES.includes(item.code as (typeof SELECTION_BAR_CODES)[number])) {
          return false;
        }
        return item.visible(selectionBarContext);
      })
    : [];

  return (
    <>
      {showSelectionBar && selectionBarContext ? (
        <Box className="tm-sftp-selection-bar">
          <Box className="tm-sftp-selection-bar-count">{tf('protocol.sftp.selectedCount', { n: selectionBarContext.selectedNames.length })}</Box>
          <Box className="tm-sftp-selection-bar-actions">
            {selectionBarItems.map(item => (
              <Box key={item.code} className={item.danger ? 'tm-sftp-selection-bar-btn is-danger' : 'tm-sftp-selection-bar-btn'} onClick={() => handleItemClick(item.code, selectionBarContext)}>
                {resolveMenuActionLabel(item.code, item.labelI18n, showHidden, mobileViewport, t)}
              </Box>
            ))}
          </Box>
        </Box>
      ) : null}
      <BodyMenuList showMenu={showMenu} localPane={localPane} showHidden={showHidden} iosPlatform={iosPlatform} onClose={closeMenu} renderMenuContext={renderMenuContext} onItemClick={handleItemClick} chmodEnabled={chmodEnabled} />
      <BodyMenuDialogs sessionId={sessionId} entries={entries} sideRef={sideRef} dialogsRef={dialogsRef} dialogContainerRef={dialogContainerRef} />
    </>
  );
}
