import { Box, ButtonBase } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { useEffect, useRef, useState } from 'react';
import { isNil, isNotNil } from '@/common/util/StringUtil.ts';
import { CodeDiff } from '@/common/components/code/CodeDiff.tsx';
import { openFileEditTab } from '@/page/common/util/fileEditOpenUtil.ts';
import { ConfirmDialog } from '@/common/components/dialog/ConfirmDialog.tsx';
import { FILE_CHANGE_STATE, type RespAiFileChangeView } from '@/common/model/resp/RespAiFileChange.ts';
import { useAiFileChangeService, useSystemUtilService, useTabManagerService } from '@/common/context/useAppHooks.tsx';

interface FileChangePageProps {
  chatId: string;
  focusPath?: string | null;
}

const STATE_KEY: Record<number, string> = {
  [FILE_CHANGE_STATE.PENDING]: 'agent.fileChange.statePending',
  [FILE_CHANGE_STATE.ACCEPTED]: 'agent.fileChange.stateAccepted',
  [FILE_CHANGE_STATE.REVERTED]: 'agent.fileChange.stateReverted',
};

const STATE_CLASS: Record<number, string> = {
  [FILE_CHANGE_STATE.PENDING]: 'tm-agent-fc-state is-pending',
  [FILE_CHANGE_STATE.ACCEPTED]: 'tm-agent-fc-state is-accepted',
  [FILE_CHANGE_STATE.REVERTED]: 'tm-agent-fc-state is-reverted',
};

function resolveViewKey(view: RespAiFileChangeView): string {
  if (view.deviceId) {
    return `${view.deviceId}:${view.path}`;
  }
  return `local:${view.path}`;
}

function resolveFileName(path: string): string {
  const parts = path.split(/[\\/]/).filter(part => part.length > 0);
  if (parts.length === 0) {
    return path;
  }
  return parts[parts.length - 1];
}

function resolveDocText(text: string | null): string {
  if (isNil(text)) {
    return '';
  }
  return text;
}

export function FileChangePage(props: FileChangePageProps) {
  const { t, tf } = useI18n();
  const { chatId, focusPath } = props;
  const focusAppliedRef = useRef(false);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const tabManagerService = useTabManagerService();
  const systemUtilService = useSystemUtilService();
  const aiFileChangeService = useAiFileChangeService();
  const [views, setViews] = useState<RespAiFileChangeView[]>([]);
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [pendingRevert, setPendingRevert] = useState<RespAiFileChangeView | null>(null);

  function resolveSelected(): RespAiFileChangeView | null {
    if (isNil(selectedKey)) {
      return null;
    }
    const found = views.find(view => resolveViewKey(view) === selectedKey);
    if (isNil(found)) {
      return null;
    }
    return found;
  }

  function applySelection(next: RespAiFileChangeView[]): void {
    if (next.length === 0) {
      setSelectedKey(null);
      return;
    }
    if (!focusAppliedRef.current && isNotNil(focusPath)) {
      focusAppliedRef.current = true;
      const focused = next.find(view => view.path === focusPath);
      if (isNotNil(focused)) {
        setSelectedKey(resolveViewKey(focused));
        return;
      }
    }
    setSelectedKey((current: string | null) => {
      if (isNotNil(current) && next.some(view => resolveViewKey(view) === current)) {
        return current;
      }
      return resolveViewKey(next[0]);
    });
  }

  async function reload(): Promise<void> {
    const next = await aiFileChangeService.selectViewByChatId(chatId);
    setViews(next);
    applySelection(next);
    setLoading(false);
  }

  function handleSelect(key: string): void {
    setSelectedKey(key);
  }

  async function handleAccept(view: RespAiFileChangeView): Promise<void> {
    setBusy(true);
    try {
      await aiFileChangeService.accept(view.chatId, view.deviceId, view.path);
      await reload();
    } finally {
      setBusy(false);
    }
  }

  function handleRevertClick(view: RespAiFileChangeView): void {
    setPendingRevert(view);
  }

  function handleCancelRevert(): void {
    setPendingRevert(null);
  }

  async function handleConfirmRevert(): Promise<void> {
    const view = pendingRevert;
    setPendingRevert(null);
    if (isNil(view)) {
      return;
    }
    setBusy(true);
    try {
      await aiFileChangeService.revert(view.chatId, view.deviceId, view.path);
      await reload();
    } finally {
      setBusy(false);
    }
  }

  function renderItem(view: RespAiFileChangeView) {
    const key = resolveViewKey(view);
    const active = key === selectedKey;
    return (
      <ButtonBase key={key} disableRipple className={active ? 'tm-agent-fc-item is-active' : 'tm-agent-fc-item'} onClick={() => handleSelect(key)}>
        <Box className="tm-agent-fc-item-head">
          <Box component="span" className="tm-agent-fc-item-name">
            {resolveFileName(view.path)}
          </Box>
          <Box component="span" className={STATE_CLASS[view.state]}>
            {t(STATE_KEY[view.state])}
          </Box>
        </Box>
        <Box className="tm-agent-fc-item-sub">
          {view.deviceId ? (
            <Box component="span" className="tm-agent-fc-item-device">
              {view.deviceId}
            </Box>
          ) : null}
          <Box component="span" className="tm-agent-fc-item-path">
            {view.path}
          </Box>
        </Box>
        <Box className="tm-agent-fc-item-meta">
          {isNil(view.baselineText) ? (
            <Box component="span" className="tm-agent-fc-created">
              {t('agent.fileChange.created')}
            </Box>
          ) : null}
          <Box component="span">{tf('agent.fileChange.edits', { n: view.editCount })}</Box>
        </Box>
      </ButtonBase>
    );
  }

  function renderHead(selected: RespAiFileChangeView) {
    return (
      <Box className="tm-agent-fc-head">
        <Box className="tm-agent-fc-head-path">{selected.path}</Box>
        <Box className="tm-agent-fc-head-actions">
          {!selected.deviceId ? (
            <ButtonBase disableRipple className="tm-agent-fc-btn is-open" onClick={() => openFileEditTab(tabManagerService, systemUtilService, selected.path)}>
              {t('fileEdit.openInEditor')}
            </ButtonBase>
          ) : null}
          {selected.state === FILE_CHANGE_STATE.PENDING ? (
            <>
              <ButtonBase disableRipple disabled={busy} className="tm-agent-fc-btn is-accept" onClick={() => void handleAccept(selected)}>
                {t('agent.fileChange.accept')}
              </ButtonBase>
              <ButtonBase disableRipple disabled={busy} className="tm-agent-fc-btn is-revert" onClick={() => handleRevertClick(selected)}>
                {t('agent.fileChange.revert')}
              </ButtonBase>
            </>
          ) : (
            <Box component="span" className={STATE_CLASS[selected.state]}>
              {t(STATE_KEY[selected.state])}
            </Box>
          )}
        </Box>
      </Box>
    );
  }

  useEffect(() => {
    void reload();
    const off = aiFileChangeService.addFileChangeListener((changedChatId: string) => {
      if (changedChatId !== chatId) {
        return;
      }
      void reload();
    });
    return off;
  }, [chatId, aiFileChangeService]);

  const selected = resolveSelected();

  return (
    <Box className="tm-agent-fc-root">
      <Box className="tm-agent-fc-left">
        <Box className="tm-agent-fc-left-title">{tf('agent.fileChange.listTitle', { n: views.length })}</Box>
        <Box className="tm-agent-fc-list">
          {views.map(view => renderItem(view))}
          {!loading && views.length === 0 ? <Box className="tm-agent-fc-empty">{t('agent.fileChange.empty')}</Box> : null}
        </Box>
      </Box>
      <Box className="tm-agent-fc-main">
        {isNotNil(selected) ? (
          <>
            {renderHead(selected)}
            {isNil(selected.currentText) && isNotNil(selected.baselineText) ? <Box className="tm-agent-fc-notice">{t('agent.fileChange.currentMissing')}</Box> : null}
            <CodeDiff fileName={resolveFileName(selected.path)} baselineText={resolveDocText(selected.baselineText)} currentText={resolveDocText(selected.currentText)} />
          </>
        ) : (
          <Box className="tm-agent-fc-empty">{t('agent.fileChange.empty')}</Box>
        )}
      </Box>
      <ConfirmDialog
        open={isNotNil(pendingRevert)}
        title={t('agent.fileChange.revert')}
        message={pendingRevert ? tf('agent.fileChange.revertMessage', { path: pendingRevert.path }) : ''}
        confirmLabel={t('agent.fileChange.revert')}
        danger
        onCancel={handleCancelRevert}
        onConfirm={() => {
          void handleConfirmRevert();
        }}
      />
    </Box>
  );
}
