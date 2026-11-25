import {Box, ButtonBase} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import type {Terminal as XTerm} from '@xterm/xterm';
import {useSystemUtilService} from '@/common/context/useAppHooks.tsx';
import {TerminalSelectionMagnifier} from './TerminalSelectionMagnifier.tsx';
import type {MagnifierTarget} from '@/page/common/util/terminalMagnifier.ts';
import {TERMINAL_HANDLE_FACE, TERMINAL_SELECT_HANDLE} from '@/common/const/Const.ts';
import {useEffect, useLayoutEffect, useRef, useState, type PointerEvent as ReactPointerEvent, type RefObject} from 'react';
import {
    clampTerminalCellToContent,
    copyTerminalSelection,
    pasteClipboardToTerminal,
    placeTerminalSelectBar,
    readHardKeyboardConnected,
    readTerminalCellMetrics,
    readTerminalSelectionAnchor,
    readTerminalSelectionEndpoint,
    readTerminalSelectionRange,
    resolveTerminalCell,
    resolveTerminalSelectionHandles,
    scrollTerminalSelectAtEdge,
    setTerminalSelectionEndpoint,
    type TerminalCell,
    type TerminalMenuPoint,
} from '@/page/common/util/terminal.ts';

export interface TerminalSelectBarProps {
    terminal: XTerm;
    onPaste: (data: string) => void;
    hostRef: RefObject<HTMLDivElement | null>;
    pasteMenuPoint?: TerminalMenuPoint | null;
    onPasteMenuChange?: (point: TerminalMenuPoint | null) => void;
}

export function TerminalSelectBar(props: TerminalSelectBarProps) {
    const {t} = useI18n();
    const [dragging, setDragging] = useState(false);
    const systemUtilService = useSystemUtilService();
    const barRef = useRef<HTMLDivElement | null>(null);
    const [magnifier, setMagnifier] = useState<MagnifierTarget | null>(null);
    const {terminal, onPaste, hostRef, pasteMenuPoint, onPasteMenuChange} = props;
    const [copyFailed, setCopyFailed] = useState(false);
    const [open, setOpen] = useState<boolean>(() => !!readTerminalSelectionRange(terminal));
    const [handles, setHandles] = useState<ReturnType<typeof resolveTerminalSelectionHandles>>(null);
    const dragRef = useRef<{ which: number; anchor: TerminalCell; pointerId: number; offsetX: number; offsetY: number; x: number; y: number; touch: boolean } | null>(null);

    const focusAfterAction = (): void => {
        if (readHardKeyboardConnected() || (document.documentElement.classList.contains('tm-keyboard-open') && terminal.textarea?.getAttribute('inputmode') !== 'none')) {
            terminal.focus();
        }
    };

    const handleCopy = (): void => {
        onPasteMenuChange?.(null);
        setCopyFailed(false);
        copyTerminalSelection(terminal, text => systemUtilService.writeClipboardText(text), () => terminal.clearSelection(), () => setCopyFailed(true));
        focusAfterAction();
    };

    const handlePaste = (): void => {
        onPasteMenuChange?.(null);
        if (terminal.hasSelection()) {
            terminal.clearSelection();
        }
        pasteClipboardToTerminal(() => systemUtilService.readClipboardText(), onPaste);
        focusAfterAction();
    };

    const handleBarPointerDown = (event: ReactPointerEvent<HTMLElement>): void => {
        event.preventDefault();
    };

    const handlePointerDown =
        (which: number) =>
            (event: ReactPointerEvent<HTMLButtonElement>): void => {
                event.preventDefault();
                event.stopPropagation();
                const host = hostRef.current;
                if (!terminal.getSelection().trim()) {
                    return;
                }
                const anchor = readTerminalSelectionAnchor(terminal, which);
                const endpoint = readTerminalSelectionEndpoint(terminal, which);
                const layout = host ? readTerminalCellMetrics(host, terminal) : null;
                if (!anchor || !endpoint || !layout) {
                    return;
                }
                event.currentTarget.setPointerCapture(event.pointerId);
                const x = layout.rowLeft + (endpoint.col + 0.5) * layout.cellW;
                const y = layout.rowTop + (endpoint.row - layout.viewportY + 0.5) * layout.cellH;
                const touch = event.pointerType !== 'mouse';
                dragRef.current = {which, anchor, pointerId: event.pointerId, offsetX: event.clientX - x, offsetY: event.clientY - y, x: event.clientX, y: event.clientY, touch};
                setDragging(true);
                if (touch) {
                    setMagnifier({...endpoint, fingerX: event.clientX, fingerY: event.clientY});
                }
            };

    const updateDrag = (): void => {
        const drag = dragRef.current;
        const host = hostRef.current;
        if (!drag || !host) {
            return;
        }
        const resolved = resolveTerminalCell(host, terminal, drag.x - drag.offsetX, drag.y - drag.offsetY);
        if (!resolved) {
            return;
        }
        const cell = clampTerminalCellToContent(terminal, resolved);
        setTerminalSelectionEndpoint(terminal, drag.anchor, cell);
        if (drag.touch) {
            setMagnifier({...cell, fingerX: drag.x, fingerY: drag.y});
        }
        const next = resolveTerminalSelectionHandles(host, terminal);
        if (!next) {
            return;
        }
        const delta = (cell.row - drag.anchor.row) * terminal.cols + cell.col - drag.anchor.col;
        const moving = delta < 0 || (delta === 0 && drag.which === TERMINAL_SELECT_HANDLE.START) ? TERMINAL_SELECT_HANDLE.START : TERMINAL_SELECT_HANDLE.END;
        setHandles(drag.which === moving ? next : {
            [TERMINAL_SELECT_HANDLE.START]: next[TERMINAL_SELECT_HANDLE.END],
            [TERMINAL_SELECT_HANDLE.END]: next[TERMINAL_SELECT_HANDLE.START],
        });
    };

    const handlePointerMove = (event: ReactPointerEvent<HTMLButtonElement>): void => {
        const drag = dragRef.current;
        if (!drag || drag.pointerId !== event.pointerId) {
            return;
        }
        event.preventDefault();
        event.stopPropagation();
        drag.x = event.clientX;
        drag.y = event.clientY;
        updateDrag();
    };

    const handlePointerUp = (event: ReactPointerEvent<HTMLButtonElement>): void => {
        if (dragRef.current?.pointerId !== event.pointerId) {
            return;
        }
        event.preventDefault();
        event.stopPropagation();
        dragRef.current = null;
        setDragging(false);
        setMagnifier(null);
        const host = hostRef.current;
        if (!host) {
            return;
        }
        const next = resolveTerminalSelectionHandles(host, terminal);
        setHandles(next);
        if (!next) {
            setOpen(false);
        }
    };

    useEffect(() => {
        if (!dragging) {
            return;
        }
        let frame = 0;
        let last = 0;
        const finish = (): void => {
            dragRef.current = null;
            setDragging(false);
            setMagnifier(null);
            const host = hostRef.current;
            if (host) {
                setHandles(resolveTerminalSelectionHandles(host, terminal));
            }
        };
        const tick = (time: number): void => {
            const drag = dragRef.current;
            const host = hostRef.current;
            if (!drag || !host) {
                return;
            }
            if (time - last >= 70) {
                last = time;
                const before = terminal.buffer.active.viewportY;
                scrollTerminalSelectAtEdge(host, terminal, drag.y - drag.offsetY);
                if (before !== terminal.buffer.active.viewportY) {
                    updateDrag();
                }
            }
            frame = requestAnimationFrame(tick);
        };
        frame = requestAnimationFrame(tick);
        window.addEventListener('blur', finish);
        window.addEventListener('resize', finish);
        return () => {
            cancelAnimationFrame(frame);
            window.removeEventListener('blur', finish);
            window.removeEventListener('resize', finish);
        };
    }, [dragging, terminal, hostRef]);

    useEffect(() => {
        const sync = (): void => {
            if (dragRef.current) {
                return;
            }
            const range = readTerminalSelectionRange(terminal);
            const host = hostRef.current;
            if (!range) {
                setOpen(false);
                setHandles(null);
                return;
            }
            setOpen(true);
            if (!host) {
                setHandles(null);
                return;
            }
            setHandles(resolveTerminalSelectionHandles(host, terminal));
        };
        sync();
        const selection = terminal.onSelectionChange(sync);
        const scroll = terminal.onScroll(sync);
        const render = terminal.onRender(sync);
        const host = hostRef.current;
        if (!host) {
            return () => {
                selection.dispose();
                scroll.dispose();
                render.dispose();
            };
        }
        const observer = new ResizeObserver(sync);
        observer.observe(host);
        return () => {
            selection.dispose();
            scroll.dispose();
            render.dispose();
            observer.disconnect();
        };
    }, [terminal, hostRef]);

    useLayoutEffect(() => {
        if (!open && !pasteMenuPoint) {
            return;
        }
        const host = hostRef.current;
        const bar = barRef.current;
        if (!host || !bar) {
            console.warn('terminal select bar missing');
            return;
        }
        const range = readTerminalSelectionRange(terminal);
        if (!range && !pasteMenuPoint) {
            setOpen(false);
            return;
        }
        placeTerminalSelectBar(host, terminal, bar, pasteMenuPoint);
    }, [open, handles, terminal, hostRef, pasteMenuPoint]);

    useEffect(() => {
        if (!pasteMenuPoint) {
            return;
        }
        dragRef.current = null;
        setDragging(false);
        setMagnifier(null);
        const dismiss = (): void => onPasteMenuChange?.(null);
        const outside = (event: PointerEvent): void => {
            if (event.target instanceof Node && !barRef.current?.contains(event.target)) {
                dismiss();
            }
        };
        const scroll = terminal.onScroll(dismiss);
        document.addEventListener('pointerdown', outside, true);
        window.addEventListener('resize', dismiss);
        window.addEventListener('blur', dismiss);
        return () => {
            scroll.dispose();
            document.removeEventListener('pointerdown', outside, true);
            window.removeEventListener('resize', dismiss);
            window.removeEventListener('blur', dismiss);
        };
    }, [pasteMenuPoint, onPasteMenuChange, terminal]);

    if (!open && !pasteMenuPoint) {
        return null;
    }
    const hasTextSelection = !pasteMenuPoint && terminal.getSelection().trim().length > 0;

    return (
        <>
            {hasTextSelection && magnifier && <TerminalSelectionMagnifier terminal={terminal} hostRef={hostRef} target={magnifier}/>}
            <Box ref={barRef} className="tm-terminal-select-bar" style={{visibility: dragging ? 'hidden' : undefined}} data-tm-gesture-ignore="true" onPointerDown={handleBarPointerDown}>
                <ButtonBase disableRipple disabled={!hasTextSelection} className="tm-terminal-select-bar-btn" onPointerDown={handleBarPointerDown} onClick={handleCopy}>
                    <span role={copyFailed ? 'alert' : undefined}>{copyFailed ? t('fatal.copyFailed') : t('common.copy')}</span>
                </ButtonBase>
                <ButtonBase disableRipple className="tm-terminal-select-bar-btn" onPointerDown={handleBarPointerDown} onClick={handlePaste}>
                    {t('common.paste')}
                </ButtonBase>
            </Box>
            {hasTextSelection && handles && handles[TERMINAL_SELECT_HANDLE.START].visible ? (
                <ButtonBase
                    disableRipple
                    className={'tm-terminal-select-handle ' + (handles[TERMINAL_SELECT_HANDLE.START].face === TERMINAL_HANDLE_FACE.LEFT ? 'is-start' : 'is-end') + (handles[TERMINAL_SELECT_HANDLE.START].above ? ' is-above' : '')}
                    data-tm-gesture-ignore="true"
                    style={{left: handles[TERMINAL_SELECT_HANDLE.START].left, top: handles[TERMINAL_SELECT_HANDLE.START].top}}
                    onPointerDown={handlePointerDown(TERMINAL_SELECT_HANDLE.START)}
                    onPointerMove={handlePointerMove}
                    onPointerUp={handlePointerUp}
                    onPointerCancel={handlePointerUp}
                    onLostPointerCapture={handlePointerUp}
                >
                    <Box className="tm-terminal-select-handle-knob"/>
                </ButtonBase>
            ) : null}
            {hasTextSelection && handles && handles[TERMINAL_SELECT_HANDLE.END].visible ? (
                <ButtonBase
                    disableRipple
                    className={'tm-terminal-select-handle ' + (handles[TERMINAL_SELECT_HANDLE.END].face === TERMINAL_HANDLE_FACE.LEFT ? 'is-start' : 'is-end') + (handles[TERMINAL_SELECT_HANDLE.END].above ? ' is-above' : '')}
                    data-tm-gesture-ignore="true"
                    style={{left: handles[TERMINAL_SELECT_HANDLE.END].left, top: handles[TERMINAL_SELECT_HANDLE.END].top}}
                    onPointerDown={handlePointerDown(TERMINAL_SELECT_HANDLE.END)}
                    onPointerMove={handlePointerMove}
                    onPointerUp={handlePointerUp}
                    onPointerCancel={handlePointerUp}
                    onLostPointerCapture={handlePointerUp}
                >
                    <Box className="tm-terminal-select-handle-knob"/>
                </ButtonBase>
            ) : null}
        </>
    );
}
