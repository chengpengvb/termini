import type { MouseEvent } from 'react';
import Add from '@mui/icons-material/Add';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { useEffect, useRef, useState } from 'react';
import MoreHoriz from '@mui/icons-material/MoreHoriz';
import { McpDetail } from '@/page/agent/mcp/McpDetail.tsx';
import { McpMarket } from '@/page/agent/mcp/McpMarket.tsx';
import { Box, ButtonBase, Typography } from '@mui/material';
import { SearchBar } from '@/page/home/components/SearchBar.tsx';
import { McpEditDialog } from '@/page/agent/mcp/McpEditDialog.tsx';
import { useBackDismiss } from '@/page/common/hooks/useBackDismiss.ts';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { AlertDialog } from '@/common/components/dialog/AlertDialog.tsx';
import { useLongPressMenu } from '@/page/common/hooks/useLongPressMenu.ts';
import { isEmpty, isNotEmpty, isNotNil } from '@/common/util/StringUtil.ts';
import type { RespAiMcpServerRuntime } from '@/common/model/AiMcpServer.ts';
import { ConfirmDialog } from '@/common/components/dialog/ConfirmDialog.tsx';
import { MobileHeaderAction } from '@/page/home/components/MobileHeader.tsx';
import { BizError, resolveResultMessage } from '@/common/model/resp/ResultModel.ts';
import { useAccountService, useAiMcpServerService } from '@/common/context/useAppHooks.tsx';
import { CommonMenu, type CommonMenuPosition } from '@/common/components/menu/CommonMenu.tsx';

const MCP_ROW_STATUS = {
  DISABLED: 0,
  CONNECTED: 1,
  ERROR: 2,
  LOADING: 3,
} as const;

const MENU_ITEM = { EDIT: 1, REFRESH: 2, TOGGLE: 3, DELETE: 4 } as const;

export function McpManage() {
  const { t, tf } = useI18n();
  const mobileViewport = usePlatformMobile();
  const accountService = useAccountService();
  const [keyword, setKeyword] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const aiMcpServerService = useAiMcpServerService();
  const pressItemRef = useRef<RespAiMcpServerRuntime | null>(null);
  const [loadingId, setLoadingId] = useState<string | null>(null);
  const [items, setItems] = useState<RespAiMcpServerRuntime[]>([]);
  const [toastError, setToastError] = useState<string | null>(null);
  const [menuPosition, setMenuPosition] = useState<CommonMenuPosition | null>(null);
  const [editing, setEditing] = useState<RespAiMcpServerRuntime | null>(null);
  const [menuItem, setMenuItem] = useState<RespAiMcpServerRuntime | null>(null);
  const [detailItem, setDetailItem] = useState<RespAiMcpServerRuntime | null>(null);
  const [pendingDelete, setPendingDelete] = useState<RespAiMcpServerRuntime | null>(null);
  const visibleItems = items.filter(item => matchInstalled(item, keyword));

  function matchInstalled(item: RespAiMcpServerRuntime, needle: string | null): boolean {
    if (isEmpty(needle)) {
      return true;
    }
    if (isEmpty(item.server.name)) {
      return false;
    }
    return item.server.name.toLowerCase().includes(needle.trim().toLowerCase());
  }

  async function reloadList(): Promise<void> {
    const list = await aiMcpServerService.listRuntime();
    setItems(list);
  }

  function handleCreate(): void {
    setEditing(null);
    setCreating(true);
  }

  function handleCloseEdit(): void {
    setCreating(false);
    setEditing(null);
  }

  async function handleSaved(): Promise<void> {
    handleCloseEdit();
    await reloadList();
  }

  function handleOpenMenu(event: MouseEvent<HTMLElement>, item: RespAiMcpServerRuntime): void {
    event.stopPropagation();
    event.preventDefault();
    setMenuPosition({ top: event.clientY, left: event.clientX });
    setMenuItem(item);
  }

  function handleContextMenu(event: MouseEvent<HTMLElement>, item: RespAiMcpServerRuntime): void {
    event.preventDefault();
    handleOpenMenu(event, item);
  }

  const longPress = useLongPressMenu((clientX, clientY) => {
    const item = pressItemRef.current;
    if (!item) {
      return;
    }
    handleContextMenu({ preventDefault: () => undefined, stopPropagation: () => undefined, clientX, clientY, currentTarget: document.body } as MouseEvent<HTMLElement>, item);
  });

  function handleInstalledMenu(event: MouseEvent<HTMLElement>, name: string): void {
    const item = items.find(candidate => candidate.server.name === name);
    if (!item) {
      return;
    }
    handleOpenMenu(event, item);
  }

  function handleCloseMenu(): void {
    setMenuPosition(null);
    setMenuItem(null);
  }

  function handleEditFromMenu(): void {
    if (!menuItem) {
      return;
    }
    const item = menuItem;
    handleCloseMenu();
    setCreating(false);
    setEditing(item);
  }

  async function handleReloadFromMenu(): Promise<void> {
    if (!menuItem) {
      return;
    }
    if (isEmpty(menuItem.server.id)) {
      return;
    }
    const id = menuItem.server.id;
    handleCloseMenu();
    setLoadingId(id);
    setToastError(null);
    try {
      await aiMcpServerService.reconnect(id);
      await reloadList();
    } catch (e) {
      await reloadList();
      if (e instanceof BizError) {
        setToastError(resolveResultMessage(e.code));
      } else if (e instanceof Error) {
        setToastError(e.message);
      } else {
        setToastError(t('agent.mcp.refreshFailed'));
      }
    } finally {
      setLoadingId(null);
    }
  }

  async function handleToggleEnabledFromMenu(): Promise<void> {
    if (!menuItem) {
      return;
    }
    if (isEmpty(menuItem.server.id)) {
      return;
    }
    const item = menuItem;
    const nextEnabled = !item.server.enabled;
    handleCloseMenu();
    setLoadingId(item.server.id);
    setToastError(null);
    try {
      await aiMcpServerService.update({
        ...item.server,
        enabled: nextEnabled,
      });
      await reloadList();
    } catch (e) {
      await reloadList();
      if (e instanceof BizError) {
        setToastError(resolveResultMessage(e.code));
      } else if (e instanceof Error) {
        setToastError(e.message);
      } else {
        setToastError(t('agent.mcp.updateFailed'));
      }
    } finally {
      setLoadingId(null);
    }
  }

  function handleDeleteFromMenu(): void {
    if (!menuItem) {
      return;
    }
    const item = menuItem;
    handleCloseMenu();
    setPendingDelete(item);
  }

  function handleCancelDelete(): void {
    setPendingDelete(null);
  }

  async function handleConfirmDelete(): Promise<void> {
    if (!pendingDelete) {
      return;
    }
    if (isEmpty(pendingDelete.server.id)) {
      return;
    }
    const id = pendingDelete.server.id;
    setPendingDelete(null);
    await aiMcpServerService.deleteById(id);
    await reloadList();
  }

  function resolveRowStatus(item: RespAiMcpServerRuntime): number {
    if (loadingId === item.server.id) {
      return MCP_ROW_STATUS.LOADING;
    }
    if (!item.server.enabled) {
      return MCP_ROW_STATUS.DISABLED;
    }
    if (item.connected) {
      return MCP_ROW_STATUS.CONNECTED;
    }
    return MCP_ROW_STATUS.ERROR;
  }

  function resolveAvatarLetter(name: string | null): string {
    if (isEmpty(name)) {
      return '?';
    }
    return name.trim().charAt(0).toUpperCase();
  }

  function resolveStatusText(item: RespAiMcpServerRuntime, status: number): string {
    if (status === MCP_ROW_STATUS.LOADING) {
      return `${item.server.name} · Loading tools`;
    }
    if (status === MCP_ROW_STATUS.DISABLED) {
      return `${item.server.name} · ${t('agent.mcp.disabled')}`;
    }
    if (status === MCP_ROW_STATUS.CONNECTED) {
      return `${item.server.name} · ${tf('agent.mcp.toolCount', { n: item.tool_count })}`;
    }
    return `${item.server.name} · ${t('agent.mcp.notConnected')}`;
  }

  function resolveStatusClass(status: number): string {
    if (status === MCP_ROW_STATUS.CONNECTED) {
      return 'tm-mcp-row-avatar-dot is-ok';
    }
    if (status === MCP_ROW_STATUS.ERROR) {
      return 'tm-mcp-row-avatar-dot is-error';
    }
    if (status === MCP_ROW_STATUS.LOADING) {
      return 'tm-mcp-row-avatar-dot is-loading';
    }
    return 'tm-mcp-row-avatar-dot is-off';
  }

  function renderMoreMenu() {
    return (
      <CommonMenu
        position={menuPosition}
        onClose={handleCloseMenu}
        items={[
          { key: MENU_ITEM.EDIT, text: t('common.edit'), onClick: handleEditFromMenu },
          {
            key: MENU_ITEM.REFRESH,
            text: t('agent.mcp.refresh'),
            onClick: () => {
              void handleReloadFromMenu();
            },
          },
          {
            key: MENU_ITEM.TOGGLE,
            text: resolveToggleLabel(),
            onClick: () => {
              void handleToggleEnabledFromMenu();
            },
          },
          { key: MENU_ITEM.DELETE, text: t('common.delete'), danger: true, onClick: handleDeleteFromMenu },
        ]}
      />
    );
  }

  function resolveToggleLabel(): string {
    if (!menuItem) {
      return t('agent.mcp.disable');
    }
    if (menuItem.server.enabled) {
      return t('agent.mcp.disable');
    }
    return t('agent.mcp.enable');
  }

  function handleOpenDetail(item: RespAiMcpServerRuntime): void {
    setDetailItem(item);
  }

  function handleBackFromDetail(): void {
    setDetailItem(null);
    void reloadList();
  }

  useBackDismiss(isNotNil(detailItem), handleBackFromDetail, 'mcp-detail');

  useEffect(() => {
    void reloadList();
  }, [aiMcpServerService]);

  useEffect(() => {
    return accountService.addCloudSyncDownloadedListener(() => {
      void reloadList();
    });
  }, [accountService]);

  return (
    <Box className="tm-mcp-manage">
      <Box className="tm-agent-page tm-page-column">
        <SearchBar placeholder={t('agent.mcp.searchPlaceholder')} searchText={keyword} onChange={setKeyword} />
        <Box className="tm-agent-section">
          <Box className={mobileViewport ? 'tm-agent-section-head is-mobile-add' : 'tm-agent-section-head'}>
            {mobileViewport ? <Typography className="tm-home-section-label">{t('agent.mcp.title')}</Typography> : <Typography className="tm-agent-section-title">{t('agent.mcp.title')}</Typography>}
            {mobileViewport ? (
              <MobileHeaderAction label={t('agent.mcp.createTitle')} onClick={handleCreate} />
            ) : (
              <ButtonBase disableRipple className="tm-agent-section-action" aria-label={t('agent.mcp.new')} onClick={handleCreate}>
                <Add sx={{ fontSize: 18 }} />
              </ButtonBase>
            )}
          </Box>
          {visibleItems.length === 0 ? (
            <Box className="tm-agent-page-empty" role={mobileViewport ? undefined : 'button'} tabIndex={mobileViewport ? undefined : 0} onClick={mobileViewport ? undefined : handleCreate}>
              <Typography className="tm-agent-page-empty-title">{t('agent.mcp.empty')}</Typography>
              {mobileViewport ? <MobileHeaderAction className="tm-empty-state-cta" label={t('agent.mcp.createTitle')} onClick={handleCreate} /> : null}
            </Box>
          ) : (
            <Box className="tm-mcp-tile-grid">
              {visibleItems.map(item => {
                const status = resolveRowStatus(item);
                return (
                  <Box
                    key={item.server.id}
                    className="tm-mcp-tile"
                    role="button"
                    tabIndex={0}
                    title={resolveStatusText(item, status)}
                    onClick={event => {
                      if (longPress.consumeIfFired()) {
                        event.preventDefault();
                        event.stopPropagation();
                        return;
                      }
                      handleOpenDetail(item);
                    }}
                    onContextMenu={event => handleContextMenu(event, item)}
                    onPointerDown={event => {
                      pressItemRef.current = item;
                      longPress.handlePointerDown(event);
                    }}
                    onPointerMove={longPress.handlePointerMove}
                    onPointerUp={longPress.handlePointerEnd}
                    onPointerCancel={longPress.handlePointerEnd}
                    onPointerLeave={longPress.handlePointerEnd}
                  >
                    <Box className="tm-mcp-row-avatar is-tile" aria-hidden>
                      <span className="tm-mcp-row-avatar-letter">{resolveAvatarLetter(item.server.name)}</span>
                      <span className={resolveStatusClass(status)} />
                    </Box>
                    <Typography className="tm-mcp-tile-name">{item.server.name}</Typography>
                    <ButtonBase
                      disableRipple
                      className="tm-mcp-tile-more"
                      aria-label={t('agent.mcp.more')}
                      disabled={status === MCP_ROW_STATUS.LOADING}
                      onClick={event => {
                        event.stopPropagation();
                        handleOpenMenu(event, item);
                      }}
                    >
                      <MoreHoriz sx={{ fontSize: 16 }} />
                    </ButtonBase>
                  </Box>
                );
              })}
            </Box>
          )}
        </Box>
        <McpMarket keyword={keyword} installedNames={items.map(item => item.server.name).filter(isNotEmpty)} onInstalled={() => void reloadList()} onInstalledMenu={handleInstalledMenu} onError={setToastError} />
      </Box>
      {renderMoreMenu()}
      {detailItem ? <McpDetail item={detailItem} onClose={handleBackFromDetail} /> : null}
      <McpEditDialog open={creating || isNotNil(editing)} server={editing ? editing.server : null} onClose={handleCloseEdit} onSaved={handleSaved} />
      <ConfirmDialog
        open={isNotNil(pendingDelete)}
        title={t('agent.mcp.deleteTitle')}
        message={pendingDelete && pendingDelete.server.name != null ? tf('agent.mcp.deleteMessage', { name: pendingDelete.server.name }) : t('agent.mcp.deleteFallback')}
        confirmLabel={t('common.delete')}
        danger
        onCancel={handleCancelDelete}
        onConfirm={() => {
          void handleConfirmDelete();
        }}
      />
      <AlertDialog duration={2000} open={isNotEmpty(toastError)} handleClose={() => setToastError(null)} message={toastError} type="error" />
    </Box>
  );
}
