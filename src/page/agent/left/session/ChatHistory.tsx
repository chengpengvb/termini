import { useI18n } from '@/common/i18n/useI18n.ts';
import { isNotNil } from '@/common/util/StringUtil.ts';
import type { AiChat } from '@/common/model/AiChat.ts';
import { useEffect, useState, type MouseEvent } from 'react';
import type { AiProject } from '@/common/model/AiProject.ts';
import { Box, Collapse, InputBase, Typography } from '@mui/material';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { HistoryItem } from '@/page/agent/left/session/HistoryItem.tsx';
import { ProjectItem } from '@/page/agent/left/session/ProjectItem.tsx';
import { EditField } from '@/page/common/components/input/EditField.tsx';
import { NavMenuItem } from '@/page/common/components/nav/NavMenuItem.tsx';
import { ConfirmDialog } from '@/common/components/dialog/ConfirmDialog.tsx';
import { ChatContextMenu } from '@/page/agent/left/session/ChatContextMenu.tsx';
import { ChevronRight, FolderOpen, History, Search } from '@mui/icons-material';
import { useChatExport } from '@/page/agent/left/session/history/useChatExport.ts';
import { useChatSearch } from '@/page/agent/left/session/history/useChatSearch.ts';
import { useChatActions } from '@/page/agent/left/session/history/useChatActions.ts';
import { ProjectContextMenu } from '@/page/agent/left/session/ProjectContextMenu.tsx';
import { useProjectActions } from '@/page/agent/left/session/history/useProjectActions.ts';
import { useChatBuckets, useChatHistoryData } from '@/page/agent/left/session/history/useChatHistoryData.ts';

interface ChatHistoryProps {
  selected: boolean;
  drawerOpen?: boolean;
  chatId: string | null;
  setChatId: (chatId: string | null) => void;
}

export function ChatHistory(props: ChatHistoryProps) {
  const { t, tf } = useI18n();
  const exporter = useChatExport();
  const data = useChatHistoryData();
  const projects = useProjectActions();
  const mobileSwipe = usePlatformMobile();
  const search = useChatSearch(data.chatList);
  const { pendingCreate, pendingProjectRename } = projects;
  const [chatsExpanded, setChatsExpanded] = useState(true);
  const { chatId, selected, drawerOpen = true, setChatId } = props;
  const [swipeOpenId, setSwipeOpenId] = useState<string | null>(null);
  const chats = useChatActions({ chatId, setChatId, onChatDeleted: closeSwipe });
  const { chatsByProject, unlinkedChats } = useChatBuckets(search.visibleChats, data.projectMap);
  const { pendingRename } = chats;

  function closeSwipe(): void {
    setSwipeOpenId(null);
  }

  function handleToggleChats(): void {
    setChatsExpanded(prev => !prev);
  }

  function handleSelectChat(id: string): void {
    closeSwipe();
    setChatId(id);
  }

  function handleContextMenu(event: MouseEvent<HTMLElement>, chat: AiChat): void {
    projects.closeMenu();
    chats.openMenu(event, chat);
  }

  function handleProjectContextMenu(event: MouseEvent<HTMLElement>, project: AiProject): void {
    chats.closeMenu();
    projects.openMenu(event, project);
  }

  function handleExportFromMenu(): void {
    if (chats.menuContext) {
      exporter.exportChat(chats.menuContext.chat);
    }
  }

  function handleCopyIdFromMenu(): void {
    if (!chats.menuContext) {
      return;
    }
    navigator.clipboard.writeText(chats.menuContext.chat.id).catch(error => {
      console.error('[agent] copy chat id error', error);
    });
  }

  function handleCreateProject(): void {
    if (chats.menuContext) {
      projects.handleCreateStart(chats.menuContext.chat);
    }
  }

  function handleMoveToProject(projectId: string): void {
    if (chats.menuContext) {
      projects.handleMoveToProject(chats.menuContext.chat, projectId);
    }
  }

  useEffect(() => {
    if (!mobileSwipe) {
      setSwipeOpenId(null);
    }
  }, [mobileSwipe]);

  useEffect(() => {
    if (!drawerOpen) {
      setSwipeOpenId(null);
      chats.closeMenu();
      projects.closeMenu();
    }
  }, [drawerOpen]);

  return (
    <Box className="tm-agent-left-section is-history">
      <Box className="tm-agent-left-section-nav tm-nav-menu">
        <NavMenuItem label={t('agent.left.projects')} icon={<FolderOpen className="tm-nav-item-icon" />} endAdornment={<ChevronRight className={projects.projectsExpanded ? 'tm-nav-item-chevron is-expanded' : 'tm-nav-item-chevron'} />} onClick={projects.handleToggleProjects} />
      </Box>
      <Collapse in={projects.projectsExpanded} timeout={220} sx={{ transitionTimingFunction: 'var(--tm-easing-ios)' }} className={projects.projectsExpanded ? 'tm-agent-left-section-collapse is-history-fill' : 'tm-agent-left-section-collapse'}>
        <Box className="tm-agent-left-section-body is-history-list">
          {data.projectList.length === 0 ? (
            <Typography className="tm-agent-left-placeholder">{t('agent.left.noProjects')}</Typography>
          ) : (
            data.projectList.map(project => (
              <ProjectItem
                key={project.id}
                project={project}
                chats={chatsByProject.get(project.id) ?? []}
                expanded={projects.expandedProjectIds.has(project.id)}
                activeChatId={chatId}
                swipeOpenChatId={swipeOpenId}
                mobileSwipe={mobileSwipe}
                onToggle={projects.handleToggleProject}
                onSelectChat={handleSelectChat}
                onChatSwipeOpen={setSwipeOpenId}
                onChatDelete={chats.handleDeleteClick}
                onChatContextMenu={handleContextMenu}
                onProjectContextMenu={handleProjectContextMenu}
              />
            ))
          )}
        </Box>
      </Collapse>
      <Box className="tm-agent-left-section-nav tm-nav-menu">
        <NavMenuItem label={t('agent.left.chats')} selected={selected} icon={<History className="tm-nav-item-icon" />} endAdornment={<ChevronRight className={chatsExpanded ? 'tm-nav-item-chevron is-expanded' : 'tm-nav-item-chevron'} />} onClick={handleToggleChats} />
      </Box>
      <Collapse in={chatsExpanded} timeout={280} sx={{ transitionTimingFunction: 'var(--tm-easing-ios)' }} className={chatsExpanded ? 'tm-agent-left-section-collapse is-history-fill' : 'tm-agent-left-section-collapse'}>
        <Box className="tm-agent-left-section-body is-history-list">
          <Box className="tm-agent-mount-search-row">
            <Search className="tm-agent-mount-search-icon" />
            <InputBase value={search.keyword} placeholder={t('agent.left.searchPlaceholder')} onChange={event => search.setKeyword(event.target.value)} className="tm-agent-mount-search" />
          </Box>
          {exporter.exportNotice !== null ? <Typography className="tm-agent-left-placeholder tm-agent-history-notice">{exporter.exportNotice}</Typography> : null}
          {unlinkedChats.length === 0 ? (
            <Typography className="tm-agent-left-placeholder">{t(search.searching ? 'agent.left.noSearchResult' : 'agent.left.noChats')}</Typography>
          ) : (
            unlinkedChats.map(chat => (
              <HistoryItem key={chat.id} chat={chat} selected={chatId === chat.id} mobileSwipe={mobileSwipe} swipeOpen={swipeOpenId === chat.id} onSelect={handleSelectChat} onSwipeOpen={setSwipeOpenId} onDeleteClick={chats.handleDeleteClick} onContextMenu={handleContextMenu} />
            ))
          )}
        </Box>
      </Collapse>
      {chats.menuContext ? (
        <ChatContextMenu
          origin={{ left: chats.menuContext.left, top: chats.menuContext.top }}
          projects={data.projectList}
          currentProjectId={data.currentProjectId(chats.menuContext.chat)}
          onRename={chats.handleRenameStart}
          onDelete={chats.handleDeleteFromMenu}
          onExport={handleExportFromMenu}
          onCopyId={handleCopyIdFromMenu}
          onCreateProject={handleCreateProject}
          onMoveToProject={handleMoveToProject}
          onClose={chats.closeMenu}
        />
      ) : null}
      {projects.projectMenuContext ? <ProjectContextMenu origin={{ left: projects.projectMenuContext.left, top: projects.projectMenuContext.top }} onRename={projects.handleRenameStart} onDelete={projects.handleDeleteFromMenu} onClose={projects.closeMenu} /> : null}
      <ConfirmDialog
        open={isNotNil(chats.pendingDelete)}
        title={t('agent.session.deleteChat')}
        message={chats.pendingDelete ? tf('agent.session.deleteChatMessage', { title: chats.pendingDelete.title }) : String()}
        confirmLabel={t('common.delete')}
        danger
        onCancel={chats.handleCancelDelete}
        onConfirm={chats.handleConfirmDelete}
      />
      <ConfirmDialog
        open={isNotNil(projects.pendingProjectDelete)}
        title={t('agent.session.deleteProject')}
        message={projects.pendingProjectDelete ? tf('agent.session.deleteProjectMessage', { name: projects.pendingProjectDelete.name }) : String()}
        confirmLabel={t('common.delete')}
        danger
        onCancel={projects.handleCancelDelete}
        onConfirm={projects.handleConfirmDelete}
      />
      <ConfirmDialog
        open={isNotNil(pendingCreate)}
        title={t('agent.session.newProject')}
        message={t('agent.session.newProjectMessage')}
        confirmLabel={t('agent.session.create')}
        confirmDisabled={pendingCreate ? pendingCreate.name.trim() === '' : true}
        onCancel={projects.handleCancelCreate}
        onConfirm={projects.handleConfirmCreate}
      >
        {pendingCreate ? (
          <Box sx={{ mt: 1.5 }}>
            <EditField autoFocus value={pendingCreate.name} onChange={name => projects.setPendingCreate({ chat: pendingCreate.chat, name })} />
          </Box>
        ) : null}
      </ConfirmDialog>
      <ConfirmDialog
        open={isNotNil(pendingRename)}
        title={t('agent.session.renameChat')}
        message={t('agent.session.renameChatMessage')}
        confirmLabel={t('common.save')}
        confirmDisabled={pendingRename ? pendingRename.name.trim() === '' : true}
        onCancel={chats.handleCancelRename}
        onConfirm={chats.handleConfirmRename}
      >
        {pendingRename ? (
          <Box sx={{ mt: 1.5 }}>
            <EditField autoFocus value={pendingRename.name} onChange={name => chats.setPendingRename({ chat: pendingRename.chat, name })} />
          </Box>
        ) : null}
      </ConfirmDialog>
      <ConfirmDialog
        open={isNotNil(pendingProjectRename)}
        title={t('agent.session.renameProject')}
        message={t('agent.session.renameProjectMessage')}
        confirmLabel={t('common.save')}
        confirmDisabled={pendingProjectRename ? pendingProjectRename.name.trim() === '' : true}
        onCancel={projects.handleCancelRename}
        onConfirm={projects.handleConfirmRename}
      >
        {pendingProjectRename ? (
          <Box sx={{ mt: 1.5 }}>
            <EditField autoFocus value={pendingProjectRename.name} onChange={name => projects.setPendingProjectRename({ project: pendingProjectRename.project, name })} />
          </Box>
        ) : null}
      </ConfirmDialog>
    </Box>
  );
}
