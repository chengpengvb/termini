import type { MouseEvent } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { Add, MoreHoriz } from '@mui/icons-material';
import type { AiSkill } from '@/common/model/AiSkill.ts';
import { createElement, useEffect, useState } from 'react';
import { Box, ButtonBase, Typography } from '@mui/material';
import { ChatIcon } from '@/assets/icon/components/ChatIcon.tsx';
import { SearchBar } from '@/page/home/components/SearchBar.tsx';
import { SkillIcon } from '@/assets/icon/components/SkillIcon.tsx';
import { SkillEditPage } from '@/page/agent/skill/SkillEditPage.tsx';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { useLongPressMenu } from '@/page/common/hooks/useLongPressMenu.ts';
import { isEmpty, isNotEmpty, isNotNil } from '@/common/util/StringUtil.ts';
import { ConfirmDialog } from '@/common/components/dialog/ConfirmDialog.tsx';
import { CommonMenu, type CommonMenuPosition } from '@/common/components/menu/CommonMenu.tsx';
import { useAccountService, useAiSkillService, useTabManagerService } from '@/common/context/useAppHooks.tsx';

const MENU_ITEM = { EDIT: 1, DELETE: 2 } as const;

interface SkillManageProps {
  onUseSkill: (skill: AiSkill) => void;
}

function matchSkill(skill: AiSkill, keyword: string | null): boolean {
  if (isEmpty(keyword)) {
    return true;
  }
  const needle = keyword.trim().toLowerCase();
  return [skill.name, skill.description].filter(isNotEmpty).join(' ').toLowerCase().includes(needle);
}

function resolveLetter(name: string | null): string {
  if (isEmpty(name)) {
    return '?';
  }
  return name.trim().charAt(0).toUpperCase();
}

function SkillManageRow(props: { skill: AiSkill; onOpen: (skill: AiSkill) => void; onUse: (event: MouseEvent<HTMLElement>, skill: AiSkill) => void; onMenu: (event: MouseEvent<HTMLElement>, skill: AiSkill) => void }) {
  const { t } = useI18n();
  const mobileViewport = usePlatformMobile();
  const { skill, onOpen, onUse, onMenu } = props;
  const longPress = useLongPressMenu((clientX, clientY) => {
    onMenu({ preventDefault: () => undefined, stopPropagation: () => undefined, clientX, clientY, currentTarget: document.body } as MouseEvent<HTMLElement>, skill);
  });

  const handleClick = (event: MouseEvent<HTMLElement>): void => {
    if (longPress.consumeIfFired()) {
      event.preventDefault();
      event.stopPropagation();
      return;
    }
    onOpen(skill);
  };

  const pressHandlers = {
    onContextMenu: (event: MouseEvent<HTMLElement>) => onMenu(event, skill),
    onPointerDown: longPress.handlePointerDown,
    onPointerMove: longPress.handlePointerMove,
    onPointerUp: longPress.handlePointerEnd,
    onPointerCancel: longPress.handlePointerEnd,
    onPointerLeave: longPress.handlePointerEnd,
  };

  const renderMobileCard = () => {
    return (
      <Box className="tm-agent-manage-card" onClick={handleClick} {...pressHandlers}>
        <Box className="tm-agent-manage-card-top">
          <Box className="tm-agent-list-row-icon" aria-hidden>
            <span>{resolveLetter(skill.name)}</span>
          </Box>
          <Box className="tm-agent-manage-card-main">
            <Typography className="tm-agent-manage-card-name">{skill.name}</Typography>
            {isNotEmpty(skill.description) ? <Typography className="tm-agent-manage-card-desc">{skill.description}</Typography> : null}
          </Box>
        </Box>
        <Box className="tm-agent-manage-card-actions">
          <ButtonBase disableRipple className="tm-snippet-card-btn tm-btn-tinted is-success" onClick={event => onUse(event, skill)}>
            <ChatIcon fontSize={18} />
            <Box component="span" className="tm-snippet-card-btn-label">
              {t('agent.skill.use')}
            </Box>
          </ButtonBase>
          <ButtonBase disableRipple className="tm-snippet-card-btn" aria-label={t('agent.mcp.more')} onClick={event => onMenu(event, skill)}>
            <MoreHoriz />
          </ButtonBase>
        </Box>
      </Box>
    );
  };

  if (mobileViewport) {
    return renderMobileCard();
  }

  return (
    <Box className="tm-agent-list-row" role="button" tabIndex={0} onClick={handleClick} {...pressHandlers}>
      <Box className="tm-agent-list-row-icon" aria-hidden>
        <span>{resolveLetter(skill.name)}</span>
      </Box>
      <Box className="tm-agent-list-row-main">
        <Typography className="tm-agent-list-row-name">{skill.name}</Typography>
        {isNotEmpty(skill.description) ? <Typography className="tm-agent-list-row-desc">{skill.description}</Typography> : null}
      </Box>
      <Box className="tm-agent-list-row-actions">
        <ButtonBase disableRipple className="tm-agent-list-row-btn" aria-label={t('agent.skill.use')} onClick={event => onUse(event, skill)}>
          <ChatIcon fontSize={16} />
        </ButtonBase>
        <ButtonBase disableRipple className="tm-agent-list-row-btn" aria-label={t('agent.mcp.more')} onClick={event => onMenu(event, skill)}>
          <MoreHoriz sx={{ fontSize: 18 }} />
        </ButtonBase>
      </Box>
    </Box>
  );
}

export function SkillManage(props: SkillManageProps) {
  const { onUseSkill } = props;
  const { t, tf } = useI18n();
  const mobileViewport = usePlatformMobile();
  const aiSkillService = useAiSkillService();
  const accountService = useAccountService();
  const tabManagerService = useTabManagerService();
  const [skills, setSkills] = useState<AiSkill[]>([]);
  const [keyword, setKeyword] = useState<string | null>(null);
  const [menuItem, setMenuItem] = useState<AiSkill | null>(null);
  const [menuPosition, setMenuPosition] = useState<CommonMenuPosition | null>(null);
  const [pendingDelete, setPendingDelete] = useState<AiSkill | null>(null);
  const visibleSkills = skills.filter(skill => matchSkill(skill, keyword));

  function sortSkills(list: AiSkill[]): AiSkill[] {
    return [...list].sort((a, b) => a.sort - b.sort);
  }

  async function reloadSkills(): Promise<void> {
    const list = await aiSkillService.selectAll();
    setSkills(sortSkills(list));
  }

  function openSkillEditTab(skill: AiSkill | null): void {
    let title = t('agent.skill.newTitle');
    if (skill) {
      if (isNotEmpty(skill.name)) {
        title = skill.name;
      } else {
        title = t('agent.skill.editTitle');
      }
    }
    tabManagerService.openTab({
      panes: [
        {
          tabBar: {
            title,
            canClose: true,
            icon: createElement(SkillIcon, { fontSize: 16 }),
          },
          content: createElement(SkillEditPage, {
            skill,
            onSaved: () => {
              void reloadSkills();
            },
          }),
        },
      ],
    });
  }

  function handleCreate(): void {
    openSkillEditTab(null);
  }

  function handleOpenMenu(event: MouseEvent<HTMLElement>, skill: AiSkill): void {
    event.stopPropagation();
    event.preventDefault();
    setMenuPosition({ top: event.clientY, left: event.clientX });
    setMenuItem(skill);
  }

  function handleCloseMenu(): void {
    setMenuPosition(null);
    setMenuItem(null);
  }

  function handleEditFromMenu(): void {
    if (!menuItem) {
      return;
    }
    const skill = menuItem;
    handleCloseMenu();
    openSkillEditTab(skill);
  }

  function handleDeleteFromMenu(): void {
    if (!menuItem) {
      return;
    }
    const skill = menuItem;
    handleCloseMenu();
    setPendingDelete(skill);
  }

  function handleCancelDelete(): void {
    setPendingDelete(null);
  }

  async function handleConfirmDelete(): Promise<void> {
    if (!pendingDelete) {
      return;
    }
    if (isEmpty(pendingDelete.id)) {
      return;
    }
    const id = pendingDelete.id;
    setPendingDelete(null);
    await aiSkillService.deleteById(id);
    await reloadSkills();
  }

  function handleUseClick(event: MouseEvent<HTMLElement>, skill: AiSkill): void {
    event.stopPropagation();
    onUseSkill(skill);
  }

  function renderMenu() {
    return (
      <CommonMenu
        position={menuPosition}
        onClose={handleCloseMenu}
        items={[
          { key: MENU_ITEM.EDIT, text: t('common.edit'), onClick: handleEditFromMenu },
          { key: MENU_ITEM.DELETE, text: t('common.delete'), danger: true, onClick: handleDeleteFromMenu },
        ]}
      />
    );
  }

  useEffect(() => {
    void reloadSkills();
  }, [aiSkillService]);

  useEffect(() => {
    return accountService.addCloudSyncDownloadedListener(() => {
      void reloadSkills();
    });
  }, [accountService]);

  return (
    <Box className="tm-agent-skill-manage">
      <Box className="tm-agent-page tm-page-column">
        <SearchBar placeholder={t('agent.skill.searchPlaceholder')} searchText={keyword} onChange={setKeyword} />
        <Box className="tm-agent-section">
          <Box className={mobileViewport ? 'tm-agent-section-head is-mobile-add' : 'tm-agent-section-head'}>
            {mobileViewport ? null : <Typography className="tm-agent-section-title">{t('agent.skill.title')}</Typography>}
            <ButtonBase disableRipple className={mobileViewport ? 'tm-mobile-header-action' : 'tm-agent-section-action'} aria-label={t('agent.skill.new')} onClick={handleCreate}>
              {mobileViewport ? (
                <>
                  <Add className="tm-mobile-header-action-icon" />
                  <Box component="span" className="tm-mobile-header-action-label">
                    {t('common.add')}
                  </Box>
                </>
              ) : (
                <Add sx={{ fontSize: 18 }} />
              )}
            </ButtonBase>
          </Box>
          {visibleSkills.length === 0 ? (
            <Box className="tm-agent-page-empty" role={mobileViewport ? undefined : 'button'} tabIndex={mobileViewport ? undefined : 0} onClick={mobileViewport ? undefined : handleCreate}>
              <Typography className="tm-agent-page-empty-title">{t('agent.skill.empty')}</Typography>
              {mobileViewport ? (
                <ButtonBase disableRipple className="tm-mobile-header-action tm-empty-state-cta" onClick={handleCreate}>
                  <Add className="tm-mobile-header-action-icon" />
                  <Box component="span" className="tm-mobile-header-action-label">
                    {t('common.add')}
                  </Box>
                </ButtonBase>
              ) : null}
            </Box>
          ) : (
            <Box className="tm-agent-list">
              {visibleSkills.map(skill => (
                <SkillManageRow key={skill.id} skill={skill} onOpen={openSkillEditTab} onUse={handleUseClick} onMenu={handleOpenMenu} />
              ))}
            </Box>
          )}
        </Box>
      </Box>
      {renderMenu()}
      <ConfirmDialog
        open={isNotNil(pendingDelete)}
        title={t('agent.skill.delete')}
        message={pendingDelete && pendingDelete.name != null ? tf('agent.skill.deleteMessage', { name: pendingDelete.name }) : t('agent.skill.deleteFallback')}
        confirmLabel={t('common.delete')}
        danger
        onCancel={handleCancelDelete}
        onConfirm={() => {
          void handleConfirmDelete();
        }}
      />
    </Box>
  );
}
