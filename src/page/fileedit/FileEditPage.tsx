import { Box, Button } from '@mui/material';
import { FolderOpen } from '@mui/icons-material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { useEffect, useRef, useState } from 'react';
import { usePaneContext } from '@/common/components/tab';
import { CodeEditor } from '@/common/components/code/CodeEditor.tsx';
import type { PaneConfig, TabConfig } from '@/common/components/tab';
import { AgentMarkdown } from '@/page/agent/common/AgentMarkdown.tsx';
import { resolveErrorText } from '@/common/model/resp/ResultModel.ts';
import { AlertDialog } from '@/common/components/dialog/AlertDialog.tsx';
import { isNil, isNotEmpty, isNotNil } from '@/common/util/StringUtil.ts';
import { PANE_DRAG_MODE } from '@/common/components/tab/model/PaneDrag.ts';
import { useLongPressMenu } from '@/page/common/hooks/useLongPressMenu.ts';
import type { RespFileEditRead } from '@/common/model/resp/RespFileEdit.ts';
import { ConfirmDialog } from '@/common/components/dialog/ConfirmDialog.tsx';
import { CommonMenu, type CommonMenuPosition } from '@/common/components/menu/CommonMenu.tsx';
import { useFileEditService, useSystemUtilService, useTabManagerService } from '@/common/context/useAppHooks.tsx';

interface FileEditPageProps {
  path: string;
}

interface PendingClose {
  dragMode: number;
  data: any;
}

const VIEW_MODE = {
  EDIT: 1,
  PREVIEW: 2,
} as const;

const MENU_ITEM = {
  REVEAL: 1,
} as const;

function isMarkdownFile(name: string): boolean {
  return /\.(md|markdown)$/i.test(name);
}

export function FileEditPage(props: FileEditPageProps) {
  const { path } = props;
  const { t, tf } = useI18n();
  const { paneId } = usePaneContext();
  const fileEditService = useFileEditService();
  const tabManagerService = useTabManagerService();
  const systemUtilService = useSystemUtilService();
  const [dirty, setDirty] = useState(false);
  const [menuPosition, setMenuPosition] = useState<CommonMenuPosition | null>(null);
  const [saving, setSaving] = useState(false);
  const [loading, setLoading] = useState(true);
  const [viewMode, setViewMode] = useState<number>(VIEW_MODE.EDIT);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [toastError, setToastError] = useState<string | null>(null);
  const [file, setFile] = useState<RespFileEditRead | null>(null);
  const [pendingClose, setPendingClose] = useState<PendingClose | null>(null);

  const docRef = useRef('');
  const savedRef = useRef('');
  const dirtyRef = useRef(false);
  const bypassCloseRef = useRef(false);
  const longPress = useLongPressMenu((clientX, clientY) => {
    setMenuPosition({ top: clientY, left: clientX });
  });

  function updateTabTitle(dirtyNow: boolean, name: string): void {
    const pane = tabManagerService.getPane(paneId);
    if (isNil(pane)) {
      return;
    }
    tabManagerService.updatePaneBar(paneId, { ...pane.tabBar, title: dirtyNow ? `● ${name}` : name });
  }

  function applyDirty(next: boolean): void {
    if (dirtyRef.current === next) {
      return;
    }
    dirtyRef.current = next;
    setDirty(next);
    if (isNotNil(file)) {
      updateTabTitle(next, file.name);
    }
  }

  function handleDocChange(doc: string): void {
    docRef.current = doc;
    applyDirty(doc !== savedRef.current);
  }

  async function handleSave(): Promise<void> {
    if (!dirtyRef.current || saving) {
      return;
    }
    setSaving(true);
    try {
      await fileEditService.write(path, docRef.current);
      savedRef.current = docRef.current;
      applyDirty(false);
    } catch (error) {
      setToastError(resolveErrorText(error));
    } finally {
      setSaving(false);
    }
  }

  function isSelfClosing(dragMode: number, data: any): boolean {
    if (dragMode === PANE_DRAG_MODE.TAB) {
      const tab = data as TabConfig;
      return tab.panes.some(pane => pane.id === paneId);
    }
    const pane = data as PaneConfig;
    return pane.id === paneId;
  }

  function handleCloseMenu(): void {
    setMenuPosition(null);
  }

  function handleReveal(): void {
    systemUtilService.revealLocalPath(path).catch((error: unknown) => {
      setToastError(resolveErrorText(error));
    });
  }

  function handleCancelClose(): void {
    setPendingClose(null);
  }

  function handleConfirmClose(): void {
    const pending = pendingClose;
    setPendingClose(null);
    if (isNil(pending)) {
      return;
    }
    bypassCloseRef.current = true;
    if (pending.dragMode === PANE_DRAG_MODE.TAB) {
      const tab = pending.data as TabConfig;
      if (isNotEmpty(tab.id)) {
        tabManagerService.closeTab(tab.id);
      }
    } else {
      tabManagerService.closePane(paneId);
    }
    bypassCloseRef.current = false;
  }

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setLoadError(null);
    fileEditService
      .read(path)
      .then(data => {
        if (cancelled) {
          return;
        }
        docRef.current = data.content;
        savedRef.current = data.content;
        setFile(data);
        setLoading(false);
      })
      .catch((error: unknown) => {
        if (cancelled) {
          return;
        }
        setLoadError(resolveErrorText(error));
        setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [path, fileEditService]);

  useEffect(() => {
    const off = tabManagerService.addCloseGuardListener((dragMode: number, data: any) => {
      if (bypassCloseRef.current || !dirtyRef.current) {
        return true;
      }
      if (!isSelfClosing(dragMode, data)) {
        return true;
      }
      setPendingClose({ dragMode, data });
      return false;
    });
    return off;
  }, [paneId, tabManagerService]);

  const markdown = isNotNil(file) && isMarkdownFile(file.name);
  const previewing = markdown && viewMode === VIEW_MODE.PREVIEW;

  return (
    <Box className="tm-fileedit-root">
      <Box className="tm-fileedit-head">
        <Box
          className="tm-fileedit-head-path"
          title={path}
          onContextMenu={longPress.handleContextMenu}
          onPointerDown={longPress.handlePointerDown}
          onPointerMove={longPress.handlePointerMove}
          onPointerUp={longPress.handlePointerEnd}
          onPointerCancel={longPress.handlePointerEnd}
          onPointerLeave={longPress.handlePointerEnd}
        >
          {path}
        </Box>
        <Box className="tm-fileedit-head-actions">
          {markdown ? (
            <>
              <Button disableRipple className={previewing ? 'tm-btn tm-btn-secondary' : 'tm-btn tm-btn-secondary is-active'} onClick={() => setViewMode(VIEW_MODE.EDIT)}>
                {t('fileEdit.edit')}
              </Button>
              <Button disableRipple className={previewing ? 'tm-btn tm-btn-secondary is-active' : 'tm-btn tm-btn-secondary'} onClick={() => setViewMode(VIEW_MODE.PREVIEW)}>
                {t('fileEdit.preview')}
              </Button>
            </>
          ) : null}
          <Button disableRipple disabled={!dirty || saving} className="tm-btn tm-btn-primary" onClick={() => void handleSave()}>
            {t('fileEdit.save')}
          </Button>
        </Box>
      </Box>
      <Box className="tm-fileedit-body">
        {isNotNil(loadError) ? <Box className="tm-fileedit-error">{loadError}</Box> : null}
        {!loading && isNil(loadError) && isNotNil(file) ? (
          previewing ? (
            <Box className="tm-fileedit-preview">
              <AgentMarkdown content={docRef.current} className="tm-agent-md tm-fileedit-doc" />
            </Box>
          ) : (
            <CodeEditor fileName={file.name} value={docRef.current} onDocChange={handleDocChange} onSave={() => void handleSave()} />
          )
        ) : null}
      </Box>
      <ConfirmDialog open={isNotNil(pendingClose)} title={t('fileEdit.closeTitle')} message={isNotNil(file) ? tf('fileEdit.closeMessage', { name: file.name }) : t('fileEdit.closeTitle')} confirmLabel={t('fileEdit.closeConfirm')} danger onCancel={handleCancelClose} onConfirm={handleConfirmClose} />
      <CommonMenu position={menuPosition} onClose={handleCloseMenu} items={[{ key: MENU_ITEM.REVEAL, text: t('agent.tool.documentReveal'), icon: <FolderOpen />, onClick: handleReveal }]} />
      <AlertDialog duration={2000} open={isNotEmpty(toastError)} handleClose={() => setToastError(null)} message={toastError} type="error" />
    </Box>
  );
}
