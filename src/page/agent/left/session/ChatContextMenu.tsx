import {useI18n} from '@/common/i18n/useI18n.ts';
import type {AiProject} from '@/common/model/AiProject.ts';
import {ArrowRight, CreateNewFolder} from '@mui/icons-material';
import {useEffect, useLayoutEffect, useRef, useState} from 'react';
import {usePlatformMobile} from '@/common/context/usePlatformMobile';
import {Box, ClickAwayListener, MenuItem, MenuList, Portal} from '@mui/material';
import {clampMenuPositionForElement} from '@/page/common/util/menuPositionUtil.ts';
import {CommonMenu, type CommonMenuItem} from '@/common/components/menu/CommonMenu.tsx';

const MENU_MIN_WIDTH = 180;
const SUBMENU_MIN_WIDTH = 200;
const SUBMENU_HIDE_MS = 120;

interface Position {
    left: number;
    top: number;
}

export interface ChatContextMenuProps {
    origin: Position;
    onClose: () => void;
    onRename: () => void;
    onDelete: () => void;
    onExport: () => void;
    onCopyId: () => void;
    projects: AiProject[];
    onCreateProject: () => void;
    currentProjectId: string | null;
    onMoveToProject: (projectId: string) => void;
}

const CHAT_MENU = {
    RENAME: 1,
    MOVE: 2,
    EXPORT: 3,
    COPY: 4,
    DELETE: 5,
    NEW_PROJECT: 6,
} as const;

export function ChatContextMenu(props: ChatContextMenuProps) {
    const {t} = useI18n();
    const moveHoldRef = useRef(false);
    const submenuHoverRef = useRef(false);
    const mobileViewport = usePlatformMobile();
    const hideTimerRef = useRef<number | null>(null);
    const [moveSheet, setMoveSheet] = useState(false);
    const menuRef = useRef<HTMLDivElement | null>(null);
    const [submenuOpen, setSubmenuOpen] = useState(false);
    const submenuRef = useRef<HTMLDivElement | null>(null);
    const moveAnchorRef = useRef<HTMLLIElement | null>(null);
    const [position, setPosition] = useState<Position | null>(null);
    const [submenuPosition, setSubmenuPosition] = useState<Position | null>(null);
    const {origin, projects, currentProjectId, onRename, onDelete, onExport, onCopyId, onCreateProject, onMoveToProject, onClose} = props;

    const attachMenu = (element: HTMLDivElement | null): void => {
        menuRef.current = element;
        if (!element) {
            return;
        }
        const next = clampMenuPositionForElement({left: origin.left, top: origin.top, element, viewportGap: 8});
        setPosition(prev => {
            if (prev && prev.left === next.left && prev.top === next.top) {
                return prev;
            }
            return next;
        });
    };

    const attachSubmenu = (element: HTMLDivElement | null): void => {
        submenuRef.current = element;
        if (!element) {
            return;
        }
        const anchor = moveAnchorRef.current;
        if (!anchor) {
            return;
        }
        const rect = anchor.getBoundingClientRect();
        const next = clampMenuPositionForElement({left: rect.right, top: rect.top, element, viewportGap: 8});
        setSubmenuPosition(prev => {
            if (prev && prev.left === next.left && prev.top === next.top) {
                return prev;
            }
            return next;
        });
    };

    const clearHideTimer = (): void => {
        if (hideTimerRef.current === null) {
            return;
        }
        window.clearTimeout(hideTimerRef.current);
        hideTimerRef.current = null;
    };

    const handleSubmenuStay = (): void => {
        submenuHoverRef.current = true;
        clearHideTimer();
        setSubmenuOpen(true);
    };

    const handleSubmenuLeave = (): void => {
        submenuHoverRef.current = false;
        clearHideTimer();
        hideTimerRef.current = window.setTimeout(() => {
            hideTimerRef.current = null;
            if (submenuHoverRef.current) {
                return;
            }
            setSubmenuOpen(false);
        }, SUBMENU_HIDE_MS);
    };

    const handleOtherItemEnter = (): void => {
        submenuHoverRef.current = false;
        clearHideTimer();
        setSubmenuOpen(false);
    };

    const handleMoveOpen = (): void => {
        moveHoldRef.current = true;
        setMoveSheet(true);
        setSubmenuOpen(true);
    };

    const handleMainSheetClose = (): void => {
        if (moveHoldRef.current) {
            moveHoldRef.current = false;
            return;
        }
        onClose();
    };

    const handleRename = (): void => {
        onRename();
        onClose();
    };

    const handleDelete = (): void => {
        onDelete();
        onClose();
    };

    const handleExport = (): void => {
        onExport();
        onClose();
    };

    const handleCopyId = (): void => {
        onCopyId();
        onClose();
    };

    const handleCreateProject = (): void => {
        onCreateProject();
        onClose();
    };

    const handleSelectProject = (projectId: string): void => {
        onMoveToProject(projectId);
        onClose();
    };

    useLayoutEffect(() => {
        if (!submenuOpen) {
            setSubmenuPosition(null);
        }
    }, [submenuOpen]);

    useEffect(() => {
        const handleKeyDown = (event: KeyboardEvent): void => {
            if (event.key !== 'Escape') {
                return;
            }
            event.preventDefault();
            onClose();
        };
        document.addEventListener('keydown', handleKeyDown, true);
        return () => {
            document.removeEventListener('keydown', handleKeyDown, true);
            clearHideTimer();
        };
    }, [onClose]);

    if (mobileViewport) {
        const mainItems: CommonMenuItem[] = [
            {key: CHAT_MENU.RENAME, text: t('agent.contextMenu.rename'), onClick: handleRename},
            {key: CHAT_MENU.MOVE, text: t('agent.contextMenu.moveToProject'), onClick: handleMoveOpen},
            {key: CHAT_MENU.EXPORT, text: t('agent.contextMenu.export'), onClick: handleExport},
            {key: CHAT_MENU.COPY, text: t('agent.contextMenu.copyId'), onClick: handleCopyId},
            {key: CHAT_MENU.DELETE, text: t('agent.contextMenu.delete'), danger: true, onClick: handleDelete},
        ];
        const moveItems: CommonMenuItem[] = [
            {key: CHAT_MENU.NEW_PROJECT, text: t('agent.contextMenu.newProject'), onClick: handleCreateProject},
            ...projects.map((project, index) => ({
                key: 100 + index,
                text: project.name,
                onClick: () => handleSelectProject(project.id),
            })),
        ];
        return (
            <>
                <CommonMenu position={moveSheet ? null : {top: origin.top, left: origin.left}} items={mainItems} onClose={handleMainSheetClose}/>
                <CommonMenu position={moveSheet ? {top: origin.top, left: origin.left} : null} items={moveItems} onClose={onClose}/>
            </>
        );
    }

    return (
        <Portal>
            <ClickAwayListener onClickAway={onClose}>
                <Box>
                    <Box
                        ref={attachMenu}
                        sx={{ position: 'fixed', left: position ? position.left: origin.left, top: position ? position.top: origin.top, zIndex: 'var(--tm-z-overlay)', minWidth: MENU_MIN_WIDTH, py: 0.5, borderRadius: 1, bgcolor: 'var(--tm-glass-dialog-bg)', border: '1px solid var(--tm-border)', boxShadow: 'var(--tm-menu-shadow)', backdropFilter: 'var(--tm-glass-dialog-blur)', WebkitBackdropFilter: 'var(--tm-glass-dialog-blur)' }}
                        onContextMenu={event => {
                            event.preventDefault();
                            event.stopPropagation();
                        }}
                    >
                        <MenuList dense disablePadding>
                            <MenuItem onMouseEnter={handleOtherItemEnter} onClick={handleRename} sx={{cursor: 'default'}}>
                                {t('agent.contextMenu.rename')}
                            </MenuItem>
                            <MenuItem ref={moveAnchorRef} onMouseEnter={handleSubmenuStay} onMouseLeave={handleSubmenuLeave} onClick={handleMoveOpen} sx={{cursor: 'default', display: 'flex', alignItems: 'center', gap: 0.5}}>
                                <Box sx={{flex: 1}}>{t('agent.contextMenu.moveToProject')}</Box>
                                <ArrowRight fontSize="small" sx={{color: 'var(--tm-text-tertiary)'}}/>
                            </MenuItem>
                            <MenuItem onMouseEnter={handleOtherItemEnter} onClick={handleExport} sx={{cursor: 'default'}}>
                                {t('agent.contextMenu.export')}
                            </MenuItem>
                            <MenuItem onMouseEnter={handleOtherItemEnter} onClick={handleCopyId} sx={{cursor: 'default'}}>
                                {t('agent.contextMenu.copyId')}
                            </MenuItem>
                            <MenuItem onMouseEnter={handleOtherItemEnter} onClick={handleDelete} sx={{cursor: 'default', color: 'var(--tm-status-danger)'}}>
                                {t('agent.contextMenu.delete')}
                            </MenuItem>
                        </MenuList>
                    </Box>
                    {submenuOpen ? (
                        <Box
                            ref={attachSubmenu}
                            sx={{ position: 'fixed', left: submenuPosition ? submenuPosition.left: origin.left, top: submenuPosition ? submenuPosition.top: origin.top, zIndex: 'var(--tm-z-overlay)', minWidth: SUBMENU_MIN_WIDTH, py: 0.5, borderRadius: 1, bgcolor: 'var(--tm-glass-dialog-bg)', border: '1px solid var(--tm-border)', boxShadow: 'var(--tm-menu-shadow)', backdropFilter: 'var(--tm-glass-dialog-blur)', WebkitBackdropFilter: 'var(--tm-glass-dialog-blur)' }}
                            onMouseEnter={handleSubmenuStay}
                            onMouseLeave={handleSubmenuLeave}
                            onContextMenu={event => {
                                event.preventDefault();
                                event.stopPropagation();
                            }}
                        >
                            <MenuList dense disablePadding>
                                <MenuItem onClick={handleCreateProject} sx={{cursor: 'default', display: 'flex', alignItems: 'center', gap: 0.75}}>
                                    <CreateNewFolder fontSize="small"/>
                                    {t('agent.contextMenu.newProject')}
                                </MenuItem>
                                {projects.map(project => (
                                    <MenuItem key={project.id} onClick={() => handleSelectProject(project.id)} sx={{cursor: 'default', color: project.id === currentProjectId ? 'var(--tm-icon-accent)' : undefined}}>
                                        {project.name}
                                    </MenuItem>
                                ))}
                            </MenuList>
                        </Box>
                    ) : null}
                </Box>
            </ClickAwayListener>
        </Portal>
    );
}
