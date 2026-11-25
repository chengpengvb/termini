import {useState, type MouseEvent} from 'react';
import {isEmpty} from '@/common/util/StringUtil.ts';
import type {AiChat} from '@/common/model/AiChat.ts';
import type {AiProject} from '@/common/model/AiProject.ts';
import {useAiChat2ProjectService, useAiProjectService} from '@/common/context/useAppHooks.tsx';

export interface ProjectMenuContext {
    project: AiProject;
    left: number;
    top: number;
}

export interface ProjectCreateContext {
    chat: AiChat;
    name: string;
}

export interface ProjectRenameContext {
    project: AiProject;
    name: string;
}

export function useProjectActions() {
    const aiProjectService = useAiProjectService();
    const aiChat2ProjectService = useAiChat2ProjectService();
    const [projectsExpanded, setProjectsExpanded] = useState(true);
    const [expandedProjectIds, setExpandedProjectIds] = useState<Set<string>>(new Set());
    const [pendingCreate, setPendingCreate] = useState<ProjectCreateContext | null>(null);
    const [pendingProjectDelete, setPendingProjectDelete] = useState<AiProject | null>(null);
    const [projectMenuContext, setProjectMenuContext] = useState<ProjectMenuContext | null>(null);
    const [pendingProjectRename, setPendingProjectRename] = useState<ProjectRenameContext | null>(null);

    function expandProject(projectId: string): void {
        setExpandedProjectIds(prev => {
            if (prev.has(projectId)) {
                return prev;
            }
            const next = new Set(prev);
            next.add(projectId);
            return next;
        });
    }

    function handleToggleProjects(): void {
        setProjectsExpanded(prev => !prev);
    }

    function handleToggleProject(id: string): void {
        setExpandedProjectIds(prev => {
            const next = new Set(prev);
            if (next.has(id)) {
                next.delete(id);
            } else {
                next.add(id);
            }
            return next;
        });
    }

    function openMenu(event: MouseEvent<HTMLElement>, project: AiProject): void {
        setProjectMenuContext({project, left: event.clientX, top: event.clientY});
    }

    function closeMenu(): void {
        setProjectMenuContext(null);
    }

    function handleRenameStart(): void {
        if (!projectMenuContext) {
            return;
        }
        setPendingProjectRename({project: projectMenuContext.project, name: projectMenuContext.project.name});
    }

    function handleCancelRename(): void {
        setPendingProjectRename(null);
    }

    function handleConfirmRename(): void {
        if (!pendingProjectRename) {
            return;
        }
        const trimmed = pendingProjectRename.name.trim();
        if (trimmed === '' || trimmed === pendingProjectRename.project.name) {
            setPendingProjectRename(null);
            return;
        }
        const projectId = pendingProjectRename.project.id;
        setPendingProjectRename(null);
        aiProjectService.rename(projectId, trimmed).catch(error => {
            console.error('Failed to rename ai project:', error);
        });
    }

    function handleDeleteFromMenu(): void {
        if (!projectMenuContext) {
            return;
        }
        setPendingProjectDelete(projectMenuContext.project);
    }

    function handleCancelDelete(): void {
        setPendingProjectDelete(null);
    }

    function handleConfirmDelete(): void {
        if (!pendingProjectDelete || isEmpty(pendingProjectDelete.id)) {
            return;
        }
        const deleteId = pendingProjectDelete.id;
        setPendingProjectDelete(null);
        aiProjectService
            .deleteById(deleteId)
            .then(() => {
                setExpandedProjectIds(prev => {
                    if (!prev.has(deleteId)) {
                        return prev;
                    }
                    const next = new Set(prev);
                    next.delete(deleteId);
                    return next;
                });
            })
            .catch(error => {
                console.error('Failed to delete ai project:', error);
            });
    }

    function handleMoveToProject(chat: AiChat, projectId: string): void {
        aiChat2ProjectService
            .setProject(chat.id, projectId)
            .then(() => expandProject(projectId))
            .catch(error => {
                console.error('Failed to move chat to project:', error);
            });
    }

    function handleCreateStart(chat: AiChat): void {
        setPendingCreate({chat, name: ''});
    }

    function handleCancelCreate(): void {
        setPendingCreate(null);
    }

    function handleConfirmCreate(): void {
        if (!pendingCreate) {
            return;
        }
        const trimmed = pendingCreate.name.trim();
        if (trimmed === '') {
            return;
        }
        const chatToMove = pendingCreate.chat;
        setPendingCreate(null);
        aiProjectService
            .create(trimmed)
            .then(project => aiChat2ProjectService.setProject(chatToMove.id, project.id).then(() => project))
            .then(project => expandProject(project.id))
            .catch(error => {
                console.error('Failed to create project:', error);
            });
    }

    return {
        projectsExpanded,
        expandedProjectIds,
        pendingCreate,
        setPendingCreate,
        pendingProjectDelete,
        projectMenuContext,
        pendingProjectRename,
        setPendingProjectRename,
        handleToggleProjects,
        handleToggleProject,
        openMenu,
        closeMenu,
        handleRenameStart,
        handleCancelRename,
        handleConfirmRename,
        handleDeleteFromMenu,
        handleCancelDelete,
        handleConfirmDelete,
        handleMoveToProject,
        handleCreateStart,
        handleCancelCreate,
        handleConfirmCreate,
    };
}
