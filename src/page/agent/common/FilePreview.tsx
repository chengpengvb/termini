import allPreset from '@file-viewer/preset-all';
import { Dialog, IconButton } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { convertFileSrc } from '@tauri-apps/api/core';
import { resolveFileName } from '@/page/common/util/fileTypeUtil.ts';
import {useBackDismiss} from '@/page/common/hooks/useBackDismiss.ts';
import { openFileEditTab } from '@/page/common/util/fileEditOpenUtil.ts';
import { useEffect, useMemo, useRef, useState, type MouseEvent } from 'react';
import { Close, Download, FolderOpen, Fullscreen, MoreHoriz } from '@mui/icons-material';
import { CommonMenu, type CommonMenuPosition } from '@/common/components/menu/CommonMenu.tsx';
import { useSystemUtilService, useTabManagerService } from '@/common/context/useAppHooks.tsx';
import FileViewer, { type FileViewerHandle, type ViewerState, type ViewerViewState } from '@file-viewer/react';

type FilePreviewState = 'generating' | 'loading' | 'ready';
type FilePreviewProps = { generating: true; path?: never } | { path: string; generating?: false };

export function FilePreview({ path, generating = false }: FilePreviewProps) {
  const { t } = useI18n();
  const byteProgress = useRef(false);
  const [percent, setPercent] = useState(0);
  const systemUtilService = useSystemUtilService();
  const tabManagerService = useTabManagerService();
  const [fullscreen, setFullscreen] = useState(false);
  const [file, setFile] = useState<File | null>(null);
  const viewer = useRef<FileViewerHandle | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const restoreView = useRef<ViewerViewState | null>(null);
  const [menu, setMenu] = useState<CommonMenuPosition | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [state, setState] = useState<FilePreviewState>(generating ? 'generating' : 'loading');
  const [theme, setTheme] = useState<'light' | 'dark'>(() => (document.documentElement.dataset.appTheme === 'light' ? 'light' : 'dark'));

  function handleViewerState(next: ViewerState): void {
    if (next.error) {
      setError(next.error instanceof Error ? next.error.message : String(next.error));
      return;
    }
    if (next.ready) {
      const api = viewer.current?.getApi();
      if (api && 'getRenderer' in api) {
        const source = api.getSource();
        if (source && !api.getRenderer(source.extension)?.load) {
          setError(`Unsupported preview: ${source.filename}`);
          return;
        }
      }
      setPercent(100);
      setState('ready');
      if (restoreView.current) {
        const savedView = restoreView.current;
        restoreView.current = null;
        void viewer.current?.applyViewState(savedView).catch(reportActionError);
      }
    }
  }

  function reportActionError(reason: unknown): void {
    setActionError(reason instanceof Error ? reason.message : String(reason));
  }

  function showContextMenu(event: MouseEvent<HTMLElement>): void {
    if (!path) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    setMenu({ left: event.clientX, top: event.clientY });
  }

  function showMore(event: MouseEvent<HTMLButtonElement>): void {
    event.stopPropagation();
    const rect = event.currentTarget.getBoundingClientRect();
    setMenu({ left: rect.left, top: rect.bottom });
  }

  function setExpanded(expanded: boolean): void {
    setMenu(null);
    restoreView.current = viewer.current?.getViewState() ?? null;
    if (file) {
      setPercent(90);
      setState('loading');
    }
    setFullscreen(expanded);
  }

  async function download(): Promise<void> {
    if (!viewer.current || !file || downloading) {
      return;
    }
    setActionError(null);
    setDownloading(true);
    try {
      await viewer.current.downloadOriginalFile();
    } catch (reason) {
      reportActionError(reason);
    } finally {
      setDownloading(false);
    }
  }

  async function reveal(): Promise<void> {
    if (!path) {
      return;
    }
    setActionError(null);
    try {
      await systemUtilService.revealLocalPath(path);
    } catch (reason) {
      reportActionError(reason);
    }
  }

  function renderViewer() {
    return (
      <span className={fullscreen ? 'tm-file-preview-content is-fullscreen' : 'tm-file-preview-content'}>
        {file ? <FileViewer ref={viewer} file={file} options={options} onStateChange={handleViewerState} style={{ visibility: state === 'ready' ? 'visible' : 'hidden' }} /> : null}
        {state !== 'ready' ? (
          <span className="tm-file-preview-pending" role="progressbar" aria-label={path ?? t('agent.tool.imageGenerateRunning')} aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.floor(percent)}>
            <span className="tm-file-preview-dots" aria-hidden="true">
              {Array.from({ length: 80 }, (_, index) => (
                <i key={index} style={{ animationDelay: `${(index % 10) * 90 + Math.floor(index / 10) * 65}ms` }} />
              ))}
            </span>
            <span className="tm-file-preview-percent">{Math.floor(percent)}%</span>
          </span>
        ) : null}
      </span>
    );
  }

  function renderHeader(expanded: boolean) {
    return (
      <span className="tm-file-preview-head">
        <span className="tm-file-preview-name" title={path}>
          {path ? resolveFileName(path) : ''}
        </span>
        <IconButton size="small" aria-label={t('fileEdit.download')} title={t('fileEdit.download')} disabled={state !== 'ready' || downloading} onClick={() => void download()}>
          <Download fontSize="small" />
        </IconButton>
        {!expanded ? (
          <IconButton size="small" aria-label={t('fileEdit.fullscreen')} title={t('fileEdit.fullscreen')} disabled={state !== 'ready'} onClick={() => setExpanded(true)}>
            <Fullscreen fontSize="small" />
          </IconButton>
        ) : null}
        <IconButton size="small" aria-label={t('common.more')} title={t('common.more')} onClick={showMore}>
          <MoreHoriz fontSize="small" />
        </IconButton>
        {expanded ? (
          <IconButton size="small" autoFocus aria-label={t('fileEdit.exitFullscreen')} title={`${t('fileEdit.exitFullscreen')} (Esc)`} onClick={() => setExpanded(false)}>
            <Close fontSize="small" />
          </IconButton>
        ) : null}
      </span>
    );
  }

  function openPath(event: MouseEvent<HTMLAnchorElement>): void {
    event.preventDefault();
    event.stopPropagation();
    if (path) {
      openFileEditTab(tabManagerService, systemUtilService, path);
    }
  }

  useEffect(() => {
    const root = document.documentElement;
    const sync = (): void => setTheme(root.dataset.appTheme === 'light' ? 'light' : 'dark');
    const observer = new MutationObserver(sync);
    observer.observe(root, { attributes: true, attributeFilter: ['data-app-theme'] });
    sync();
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    const controller = new AbortController();
    byteProgress.current = false;
    setFile(null);
    setError(null);
    setActionError(null);
    setMenu(null);
    setPercent(0);
    setState(generating ? 'generating' : 'loading');
    if (generating || !path) {
      return () => controller.abort();
    }
    const sourcePath = path;

    async function load(): Promise<void> {
      const url = /^https?:\/\//i.test(sourcePath) ? sourcePath : convertFileSrc(sourcePath);
      const response = await fetch(url, { signal: controller.signal });
      if (!response.ok) {
        throw new Error(`${response.status} ${response.statusText}`);
      }
      const total = Number(response.headers.get('content-length'));
      const reader = response.body?.getReader();
      let data: Blob;
      if (reader) {
        const chunks: Uint8Array<ArrayBuffer>[] = [];
        let loaded = 0;
        try {
          while (true) {
            const chunk = await reader.read();
            if (chunk.done) {
              break;
            }
            chunks.push(new Uint8Array(chunk.value));
            loaded += chunk.value.byteLength;
            if (total > 0) {
              byteProgress.current = true;
              setPercent(Math.min(99, (loaded / total) * 100));
            }
          }
        } finally {
          reader.releaseLock();
        }
        data = new Blob(chunks, { type: response.headers.get('content-type') ?? '' });
      } else {
        data = await response.blob();
      }
      if (controller.signal.aborted) {
        return;
      }
      setFile(new File([data], resolveFileName(sourcePath), { type: data.type }));
    }

    void load().catch(reason => {
      if (!controller.signal.aborted) {
        setError(reason instanceof Error ? reason.message : String(reason));
      }
    });
    return () => controller.abort();
  }, [path, generating]);

  useEffect(() => {
    if (state === 'ready' || error) {
      return;
    }
    const timer = window.setInterval(() => {
      if (!byteProgress.current) {
        setPercent(current => Math.min(90, current + (90 - current) * 0.035));
      }
    }, 400);
    return () => window.clearInterval(timer);
  }, [state, error]);

  const options = useMemo(
    () => ({
      preset: allPreset,
      theme,
      fit: 'contain' as const,
      toolbar: false,
      pdf: { toolbar: false, zoomToolbar: false, navigation: false },
      ui: { density: 'compact' as const },
    }),
    [theme],
  );

  useBackDismiss(fullscreen, () => setExpanded(false));
  useBackDismiss(menu !== null, () => setMenu(null));

  if (error && path) {
    return (
      <a className="tm-agent-md-link" title={`${path}\n${error}`} href={`file://${path}`} onClick={openPath}>
        {path}
      </a>
    );
  }

  return (
    <>
      <span className="tm-file-preview" data-preview-state={state} onContextMenuCapture={showContextMenu}>
        {path ? renderHeader(false) : null}
        {!fullscreen ? renderViewer() : null}
        {path ? (
          <span className="tm-file-preview-hint" role={actionError ? 'alert' : undefined} title={actionError ?? t('fileEdit.previewActions')}>
            {actionError ?? t('fileEdit.previewActions')}
          </span>
        ) : null}
      </span>
      <Dialog fullScreen open={fullscreen} onClose={() => setExpanded(false)} aria-label={path ? resolveFileName(path) : t('fileEdit.fullscreen')} slotProps={{ paper: { className: 'tm-file-preview-dialog' } }}>
        <div className="tm-file-preview-page" onContextMenuCapture={showContextMenu}>
          {renderHeader(true)}
          {fullscreen ? renderViewer() : null}
          <span className="tm-file-preview-hint" role={actionError ? 'alert' : undefined}>
            {actionError ?? t('fileEdit.previewActions')}
          </span>
        </div>
      </Dialog>
      <CommonMenu
        position={menu}
        onClose={() => setMenu(null)}
        items={[
          { key: 1, text: t('fileEdit.download'), icon: <Download fontSize="small" />, disabled: state !== 'ready' || downloading, onClick: () => void download() },
          { key: 2, text: t('fileEdit.showInFolder'), icon: <FolderOpen fontSize="small" />, disabled: !path || /^https?:\/\//i.test(path), onClick: () => void reveal() },
          { key: 3, text: t(fullscreen ? 'fileEdit.exitFullscreen' : 'fileEdit.fullscreen'), icon: <Fullscreen fontSize="small" />, disabled: state !== 'ready', onClick: () => setExpanded(!fullscreen) },
        ]}
      />
    </>
  );
}
