import {useEffect} from 'react';
import type {Terminal as XTerm} from '@xterm/xterm';
import {useSystemUtilService} from '@/common/context/useAppHooks.tsx';
import {copyTerminalSelection, pasteClipboardToTerminal} from '@/page/common/util/terminal.ts';

export interface TerminalShortcutsProps {
    terminal: XTerm;
    onPaste: (data: string) => void;
    registerKeyHandler: (handler: (event: KeyboardEvent) => boolean) => (() => void);
}

function isMacPlatform(): boolean {
    return /Mac|iPhone|iPad|iPod/.test(navigator.platform || navigator.userAgent);
}

export function TerminalShortcuts(props: TerminalShortcutsProps) {
    const {terminal, onPaste, registerKeyHandler} = props;
    const systemUtilService = useSystemUtilService();

    useEffect(() => {
        const isMac = isMacPlatform();

        const copySelection = (): boolean => {
            return copyTerminalSelection(terminal, text => systemUtilService.writeClipboardText(text));
        };

        const pasteClipboard = (): void => {
            pasteClipboardToTerminal(() => systemUtilService.readClipboardText(), onPaste);
        };

        const handleKeyEvent = (event: KeyboardEvent): boolean => {
            if (event.type !== 'keydown' || event.altKey) {
                return true;
            }
            const ctrlOrMeta = isMac ? event.metaKey : event.ctrlKey;
            if (event.key === 'Insert' && event.shiftKey && !ctrlOrMeta) {
                event.preventDefault();
                event.stopPropagation();
                pasteClipboard();
                return false;
            }
            if (!ctrlOrMeta) {
                return true;
            }
            const key = event.key.toLowerCase();
            if (key === 'insert' && copySelection()) {
                event.preventDefault();
                event.stopPropagation();
                return false;
            }
            if (key === 'v') {
                if (!event.shiftKey) {
                    return true;
                }
                event.preventDefault();
                event.stopPropagation();
                pasteClipboard();
                return false;
            }
            if (key !== 'c') {
                return true;
            }
            if (isMac) {
                event.preventDefault();
                event.stopPropagation();
                copySelection();
                return false;
            }
            if (copySelection()) {
                event.preventDefault();
                event.stopPropagation();
                return false;
            }
            if (event.shiftKey) {
                event.preventDefault();
                event.stopPropagation();
            }
            return !event.shiftKey;
        };

        return registerKeyHandler(handleKeyEvent);
    }, [terminal, onPaste, systemUtilService, registerKeyHandler]);

    return null;
}
