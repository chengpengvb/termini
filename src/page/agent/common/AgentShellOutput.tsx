import {Box} from '@mui/material';
import '@xterm/xterm/css/xterm.css';
import {FitAddon} from '@xterm/addon-fit';
import {Terminal as XTerm} from '@xterm/xterm';
import {isNil} from '@/common/util/StringUtil.ts';
import {useEffect, useRef, useState, type ReactNode} from 'react';
import {useTerminalThemeService} from '@/common/context/useAppHooks.tsx';
import {resolveTerminalFontFamily, resolveTerminalTheme, stampXtermScreen} from '@/page/common/util/terminal.ts';

interface AgentShellOutputProps {
    content: string;
    active: boolean;
}

const SHELL_OUTPUT_MAX_ROWS = 24;
const SHELL_OUTPUT_FONT_SIZE = 12.5;
const SHELL_OUTPUT_LINE_HEIGHT = 1.35;

function resolveHostHeight(rows: number): number {
    return Math.ceil(rows * SHELL_OUTPUT_FONT_SIZE * SHELL_OUTPUT_LINE_HEIGHT);
}

export function AgentShellOutput(props: AgentShellOutputProps): ReactNode {
    const {content, active} = props;
    const writtenRef = useRef<string>('');
    const terminalRef = useRef<XTerm | null>(null);
    const fitAddonRef = useRef<FitAddon | null>(null);
    const appliedThemeRef = useRef<string | null>(null);
    const terminalThemeService = useTerminalThemeService();
    const containerRef = useRef<HTMLDivElement | null>(null);
    const [hostHeight, setHostHeight] = useState(() => resolveHostHeight(1));

    function isTerminalAlive(terminal: XTerm): boolean {
        if (terminalRef.current !== terminal) {
            return false;
        }
        return !isNil(terminal.element);
    }

    function resolveCols(terminal: XTerm): number {
        const fitAddon = fitAddonRef.current;
        if (isNil(fitAddon)) {
            return terminal.cols;
        }
        const proposed = fitAddon.proposeDimensions();
        if (proposed && proposed.cols > 0) {
            return proposed.cols;
        }
        return terminal.cols;
    }

    function syncTerminalSize(): void {
        const terminal = terminalRef.current;
        if (isNil(terminal)) {
            return;
        }
        if (!isTerminalAlive(terminal)) {
            return;
        }
        const cols = resolveCols(terminal);
        const buffer = terminal.buffer.active;
        let rows = buffer.baseY + buffer.cursorY + 1;
        if (rows < 1) {
            rows = 1;
        }
        if (rows > SHELL_OUTPUT_MAX_ROWS) {
            rows = SHELL_OUTPUT_MAX_ROWS;
        }
        const nextHeight = resolveHostHeight(rows);
        setHostHeight((prev: number) => prev === nextHeight ? prev : nextHeight);
        if (terminal.cols === cols && terminal.rows === rows) {
            return;
        }
        terminal.resize(cols, rows);
    }

    function scrollTerminalBottom(terminal: XTerm): void {
        if (!isTerminalAlive(terminal)) {
            return;
        }
        terminal.scrollToBottom();
    }

    function writeContent(terminal: XTerm, next: string): void {
        const previous = writtenRef.current;
        if (next === previous) {
            return;
        }

        function afterWrite(): void {
            if (!isTerminalAlive(terminal)) {
                return;
            }
            syncTerminalSize();
            scrollTerminalBottom(terminal);
        }

        if (next.startsWith(previous)) {
            writtenRef.current = next;
            terminal.write(next.slice(previous.length), afterWrite);
            return;
        }
        writtenRef.current = next;
        terminal.reset();
        terminal.write(next, afterWrite);
    }

    useEffect(() => {
        const container = containerRef.current;
        if (isNil(container)) {
            return;
        }
        if (terminalRef.current) {
            return;
        }
        const terminal = new XTerm({
            rows: 1,
            convertEol: true,
            disableStdin: true,
            cursorBlink: false,
            fontSize: SHELL_OUTPUT_FONT_SIZE,
            lineHeight: SHELL_OUTPUT_LINE_HEIGHT,
            scrollback: 5000,
            theme: resolveTerminalTheme(),
            fontFamily: resolveTerminalFontFamily(),
        });
        const fitAddon = new FitAddon();
        fitAddonRef.current = fitAddon;
        terminalRef.current = terminal;
        terminal.loadAddon(fitAddon);
        terminal.open(container);
        stampXtermScreen(container);
        const observer = new ResizeObserver(() => {
            syncTerminalSize();
        });
        observer.observe(container);
        return () => {
            observer.disconnect();
            terminal.dispose();
            terminalRef.current = null;
            fitAddonRef.current = null;
            writtenRef.current = '';
        };
    }, []);

    useEffect(() => {
        const terminal = terminalRef.current;
        if (isNil(terminal)) {
            return;
        }
        writeContent(terminal, content);
        if (active) {
            syncTerminalSize();
            scrollTerminalBottom(terminal);
        }
    }, [content, active]);

    useEffect(() => {
        return terminalThemeService.addChangeListener(() => {
            const terminal = terminalRef.current;
            const themeId = document.documentElement.getAttribute('data-app-theme');
            if (isNil(terminal) || appliedThemeRef.current === themeId) {
                return;
            }
            appliedThemeRef.current = themeId;
            terminal.options.theme = resolveTerminalTheme();
        });
    }, [terminalThemeService]);

    return (
        <Box className="tm-agent-shell-output">
            <Box ref={containerRef} className="tm-agent-shell-output-xterm" data-tm-terminal="true" style={{height: hostHeight}}/>
        </Box>
    );
}
