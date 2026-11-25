import type { MouseEvent } from 'react';
import { useEffect, useState } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { isNotEmpty } from '@/common/util/StringUtil.ts';
import type { Snippet } from '@/common/model/Snippet.ts';
import { Box, ButtonBase, Typography } from '@mui/material';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { useLongPressMenu } from '@/page/common/hooks/useLongPressMenu.ts';
import { ConfirmDialog } from '@/common/components/dialog/ConfirmDialog.tsx';
import { useSnippetService, useSystemUtilService } from '@/common/context/useAppHooks.tsx';
import { CommonMenu, type CommonMenuPosition } from '@/common/components/menu/CommonMenu.tsx';
import { Check, ContentCopy, Delete, DeleteOutline, Edit, EditOutlined, MoreHoriz } from '@mui/icons-material';

const MENU_ITEM = { EDIT: 1, DELETE: 2 } as const;

const PROMPT_MARK = (
  <svg viewBox="0 0 16 16" aria-hidden="true">
    <path d="M3.2 4.4 7.2 8 3.2 11.6" fill="none" stroke="currentColor" strokeWidth="1.45" strokeLinecap="round" strokeLinejoin="round" />
    <path d="M8.4 12.1h4.4" fill="none" stroke="currentColor" strokeWidth="1.45" strokeLinecap="round" />
  </svg>
);

export function Item(props: { snippet: Snippet; onEdit: (value: Snippet) => void; onDeleted: () => void }) {
  const { t, tf } = useI18n();
  const mobileViewport = usePlatformMobile();
  const snippetService = useSnippetService();
  const { snippet, onEdit, onDeleted } = props;


  
  const systemUtilService = useSystemUtilService();
  const [copied, setCopied] = useState<boolean>(false);
  const [confirmDeleteOpen, setConfirmDeleteOpen] = useState<boolean>(false);
  const [menuPosition, setMenuPosition] = useState<CommonMenuPosition | null>(null);
  const longPress = useLongPressMenu((clientX, clientY) => {
    setMenuPosition({ top: clientY, left: clientX });
  });

  function handleEdit(): void {
    onEdit(snippet);
  }

  function handleMenuClick(event: MouseEvent<HTMLElement>): void {
    event.stopPropagation();
    setMenuPosition({ top: event.clientY, left: event.clientX });
  }

  function handleClick(event: MouseEvent<HTMLElement>): void {
    if (longPress.consumeIfFired()) {
      event.preventDefault();
      event.stopPropagation();
      return;
    }
    handleEdit();
  }

  function handleCloseMenu(): void {
    setMenuPosition(null);
  }

  function handleDelete(): void {
    setConfirmDeleteOpen(true);
  }

  function handleCancelDelete(): void {
    setConfirmDeleteOpen(false);
  }

  async function handleConfirmDelete(): Promise<void> {
    setConfirmDeleteOpen(false);
    await snippetService.deleteById(snippet.id);
    onDeleted();
  }

  async function handleCopy(event: MouseEvent<HTMLElement>): Promise<void> {
    event.stopPropagation();
    await systemUtilService.writeClipboardText(snippet.command);
    setCopied(true);
  }

  function handleActionClick(event: MouseEvent<HTMLElement>, action: () => void): void {
    event.stopPropagation();
    action();
  }

  const longPressHandlers = {
    onContextMenu: longPress.handleContextMenu,
    onPointerDown: longPress.handlePointerDown,
    onPointerMove: longPress.handlePointerMove,
    onPointerUp: longPress.handlePointerEnd,
    onPointerCancel: longPress.handlePointerEnd,
    onPointerLeave: longPress.handlePointerEnd,
  };

  const renderMobileCard = () => {
    return (
      <Box className="tm-snippet-card" onClick={handleClick} {...longPressHandlers}>
        <Typography className="tm-snippet-card-name tm-ellipsis">{snippet.name}</Typography>
        <Box className="tm-snippet-card-command">{snippet.command}</Box>
        {isNotEmpty(snippet.remark) ? <Typography className="tm-snippet-card-remark">{snippet.remark}</Typography> : null}
        <Box className="tm-snippet-card-actions">
          <ButtonBase disableRipple className="tm-snippet-card-btn tm-btn-tinted is-success" onClick={event => void handleCopy(event)}>
            {copied ? <Check /> : <ContentCopy />}
            <Box component="span" className="tm-snippet-card-btn-label">
              {copied ? t('agent.copy.copied') : t('common.copy')}
            </Box>
          </ButtonBase>
          <ButtonBase disableRipple className="tm-snippet-card-btn" aria-label={t('common.edit')} onClick={event => handleActionClick(event, handleEdit)}>
            <EditOutlined />
          </ButtonBase>
          <ButtonBase disableRipple className="tm-snippet-card-btn tm-btn-tinted is-danger" aria-label={t('common.delete')} onClick={event => handleActionClick(event, handleDelete)}>
            <DeleteOutline />
          </ButtonBase>
        </Box>
      </Box>
    );
  };

  const renderRow = () => {
    return (
      <Box role="button" tabIndex={0} className="tm-agent-list-row tm-snippet-item" onClick={handleClick} {...longPressHandlers}>
        <Box className="tm-agent-list-row-icon" aria-hidden>
          {PROMPT_MARK}
        </Box>
        <Box className="tm-agent-list-row-main">
          <Typography className="tm-agent-list-row-name">{snippet.name}</Typography>
          <Typography className="tm-agent-list-row-desc is-mono">{snippet.command}</Typography>
        </Box>
        <Box className="tm-agent-list-row-actions">
          <ButtonBase disableRipple className="tm-agent-list-row-btn" aria-label={t('common.edit')} onClick={handleMenuClick}>
            <MoreHoriz sx={{ fontSize: 18 }} />
          </ButtonBase>
        </Box>
      </Box>
    );
  };

  useEffect(() => {
    if (!copied) {
      return;
    }
    const timer = window.setTimeout(() => {
      setCopied(false);
    }, 1500);
    return () => {
      window.clearTimeout(timer);
    };
  }, [copied]);

  return (
    <>
      {mobileViewport ? renderMobileCard() : renderRow()}
      <CommonMenu
        position={menuPosition}
        onClose={handleCloseMenu}
        items={[
          { key: MENU_ITEM.EDIT, text: t('common.edit'), icon: <Edit />, onClick: handleEdit },
          { key: MENU_ITEM.DELETE, text: t('common.delete'), icon: <Delete />, danger: true, onClick: handleDelete },
        ]}
      />
      <ConfirmDialog
        open={confirmDeleteOpen}
        title={t('home.snippet.deleteTitle')}
        message={tf('home.snippet.deleteMessage', { name: snippet.name })}
        confirmLabel={t('common.delete')}
        danger
        onCancel={handleCancelDelete}
        onConfirm={() => {
          void handleConfirmDelete();
        }}
      />
    </>
  );
}
