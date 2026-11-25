import {useEffect, type RefObject} from 'react';
import {usePaneContext} from '@/common/components/tab';
import {useTabManagerService} from '@/common/context/useAppHooks.tsx';

interface InlineRevertEscapeParams {
    editing: boolean;
    submitting: boolean;
    onCancel: () => void;
    wrapRef: RefObject<HTMLDivElement | null>;
}

export function useInlineRevertEscape(params: InlineRevertEscapeParams): void {
    const {paneId} = usePaneContext();
    const manager = useTabManagerService();
    const {editing, submitting, onCancel, wrapRef} = params;

    useEffect(() => {
        if (!editing || submitting) {
            return;
        }
        function onKeyDown(event: KeyboardEvent): void {
            if (event.key !== 'Escape' || event.defaultPrevented || event.isComposing || event.keyCode === 229) {
                return;
            }
            const tabId = manager.getActiveTabId();
            const tab = tabId ? manager.getTab(tabId) : null;
            if (!tab || !tab.panes.some(pane => pane.id === paneId) || (tab.focusedPaneId && tab.focusedPaneId !== paneId)) {
                return;
            }
            const wrap = wrapRef.current;
            if (!wrap || !wrap.isConnected || wrap.getClientRects().length === 0) {
                return;
            }
            const overlayOpen = [...document.querySelectorAll('[role="dialog"], [role="menu"], [role="listbox"], [data-tm-overlay="true"]')].some(element => {
                const rect = element.getBoundingClientRect();
                const visibility = getComputedStyle(element).visibility;
                return visibility !== 'hidden' && visibility !== 'collapse' && rect.width > 0 && rect.height > 0 && rect.bottom > 0 && rect.right > 0 && rect.top < innerHeight && rect.left < innerWidth;
            });
            if (overlayOpen) {
                return;
            }
            const target = event.target;
            if (target instanceof Element && !wrap.contains(target) && target.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"]), [role="textbox"], .xterm, .monaco-editor, .cm-editor')) {
                return;
            }
            event.preventDefault();
            event.stopPropagation();
            onCancel();
        }
        document.addEventListener('keydown', onKeyDown, true);
        return () => document.removeEventListener('keydown', onKeyDown, true);
    }, [editing, submitting, manager, paneId, onCancel, wrapRef]);
}
