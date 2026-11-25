import {translate} from '@/common/i18n/i18n.ts';
import {isRdpKeyboardTarget} from '@/page/common/util/rdpUtil.ts';
import type {SystemUtilService} from '@/service/SystemUtilService.ts';
import {getServiceContainer} from '@/common/context/ServiceContext.tsx';
import {SystemUtilServiceImpl} from '@/service/impl/SystemUtilServiceImpl.ts';

const BLOCKED_CTRL_KEYS = new Set([
    'f',
    'g',
    'p',
    's',
    'u',
    'j',
    'h',
    'd',
    'l',
    't',
    'n',
    'w',
    'o',
    'b',
]);

const EDITOR_PLAIN_KEYS = new Set(['Tab', 'F3']);

const EDITOR_CTRL_KEYS = new Set(['s', 'f', 'g']);

const BLOCKED_FUNCTION_KEYS = new Set(['F1', 'F3', 'F5', 'F7']);

const MOBILE_UA = /Android|iPhone|iPad/i;

interface ContextMenuItem {
    label: string;
    onAction: () => void;
}

let menuElement: HTMLDivElement | null = null;

function isEditorTarget(e: KeyboardEvent): boolean {
    return e.target instanceof Element && e.target.closest('[data-tm-editor="true"]') !== null;
}

function isEditableTarget(target: EventTarget | null): boolean {
    if (!(target instanceof Element)) {
        return false;
    }
    return target.closest('input, textarea, [contenteditable]:not([contenteditable="false"])') !== null;
}

function isTerminalTarget(target: EventTarget | null): boolean {
    return target instanceof Element && target.closest('[data-tm-terminal="true"]') !== null;
}

function isFormNavigationTarget(target: EventTarget | null): boolean {
    if (!(target instanceof Element)) {
        return false;
    }
    if (isTerminalTarget(target)) {
        return false;
    }
    return target.closest('[data-tm-overlay="true"]') !== null || isEditableTarget(target);
}

function hasCopyableSelection(target: EventTarget | null): boolean {
    if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) {
        return target.selectionStart !== null && target.selectionEnd !== null && target.selectionStart !== target.selectionEnd;
    }
    const selection = window.getSelection();
    if (!selection) {
        return false;
    }
    return !selection.isCollapsed;
}

function closeContextMenu(): void {
    if (!menuElement) {
        return;
    }
    menuElement.remove();
    menuElement = null;
}

async function pasteIntoFocused(): Promise<void> {
    const text = await getServiceContainer().resolve<SystemUtilService>(SystemUtilServiceImpl).readClipboardText();
    if (!text) {
        return;
    }
    document.execCommand('insertText', false, text);
}

function buildMenuItems(target: EventTarget | null): ContextMenuItem[] {
    const items: ContextMenuItem[] = [];
    if (hasCopyableSelection(target)) {
        items.push({
            label: translate('common.copy'),
            onAction: () => {
                document.execCommand('copy');
            },
        });
    }
    if (isEditableTarget(target)) {
        items.push({
            label: translate('common.paste'),
            onAction: () => {
                pasteIntoFocused().catch((error) => {
                    console.error('[BrowserShortcut] paste failed:', error);
                });
            },
        });
    }
    return items;
}

function showContextMenu(e: MouseEvent): void {
    closeContextMenu();
    const items = buildMenuItems(e.target);
    if (items.length === 0) {
        return;
    }
    const menu = document.createElement('div');
    menu.className = 'tm-ctx-menu';
    for (const item of items) {
        const node = document.createElement('button');
        node.type = 'button';
        node.className = 'tm-ctx-menu-item';
        node.textContent = item.label;
        node.addEventListener('mousedown', (event) => {
            event.preventDefault();
        });
        node.addEventListener('click', () => {
            closeContextMenu();
            item.onAction();
        });
        menu.appendChild(node);
    }
    document.body.appendChild(menu);
    const rect = menu.getBoundingClientRect();
    menu.style.left = `${Math.max(0, Math.min(e.clientX, window.innerWidth - rect.width - 4))}px`;
    menu.style.top = `${Math.max(0, Math.min(e.clientY, window.innerHeight - rect.height - 4))}px`;
    menuElement = menu;
}

export function installBrowserShortcutBlocker(): void {
    const onKeyDown = (e: KeyboardEvent): void => {
        if (isRdpKeyboardTarget(e.target)) {
            return;
        }
        if (e.key === 'Escape') {
            closeContextMenu();
        }
        if (EDITOR_PLAIN_KEYS.has(e.key) && isEditorTarget(e)) {
            return;
        }
        if (e.key === 'Tab') {
            if (!isFormNavigationTarget(e.target)) {
                e.preventDefault();
            }
            return;
        }
        if (BLOCKED_FUNCTION_KEYS.has(e.key)) {
            e.preventDefault();
            return;
        }
        const ctrlOrMeta = e.ctrlKey || e.metaKey;
        if (!ctrlOrMeta) {
            return;
        }
        if (e.altKey) {
            return;
        }
        const key = e.key.toLowerCase();
        if (BLOCKED_CTRL_KEYS.has(key)) {
            if (EDITOR_CTRL_KEYS.has(key) && isEditorTarget(e)) {
                return;
            }
            e.preventDefault();
        }
    };

    const onContextMenu = (e: Event): void => {
        if (MOBILE_UA.test(navigator.userAgent)) {
            return;
        }
        if (isTerminalTarget(e.target)) {
            return;
        }
        e.preventDefault();
        if (e instanceof MouseEvent) {
            showContextMenu(e);
        }
    };

    const onDismiss = (e: Event): void => {
        if (!menuElement) {
            return;
        }
        if (e.target instanceof Node && menuElement.contains(e.target)) {
            return;
        }
        closeContextMenu();
    };

    window.addEventListener('keydown', onKeyDown, {capture: true});
    window.addEventListener('contextmenu', onContextMenu, {capture: true});
    window.addEventListener('mousedown', onDismiss, {capture: true});
    window.addEventListener('scroll', onDismiss, {capture: true});
    window.addEventListener('blur', () => {
        closeContextMenu();
    });
}
