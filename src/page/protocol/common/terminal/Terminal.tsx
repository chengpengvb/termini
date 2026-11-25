import '@xterm/xterm/css/xterm.css';
import {FitAddon} from '@xterm/addon-fit';
import {WebglAddon} from '@xterm/addon-webgl';
import Close from '@mui/icons-material/Close';
import {Terminal as XTerm} from '@xterm/xterm';
import {SearchAddon} from '@xterm/addon-search';
import {TerminalDrop} from './TerminalDrop.tsx';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {CommandSuggest} from './CommandSuggest.tsx';
import {TerminalSearch} from './TerminalSearch.tsx';
import {TerminalSnippet} from './TerminalSnippet.tsx';
import {Unicode11Addon} from '@xterm/addon-unicode11';
import {usePaneContext} from '@/common/components/tab';
import {Box, ButtonBase, Typography} from '@mui/material';
import {TerminalBroadcast} from './TerminalBroadcast.tsx';
import {TerminalSelectBar} from './TerminalSelectBar.tsx';
import {TerminalShortcuts} from './TerminalShortcuts.tsx';
import {WindowSize} from '@/common/model/dto/WindowSize.ts';
import {TerminalSettingsPanel} from './TerminalSettingsPanel.tsx';
import {useBackDismiss} from '@/page/common/hooks/useBackDismiss.ts';
import {usePlatformMobile} from '@/common/context/usePlatformMobile';
import {useDocumentClass} from '@/page/common/hooks/useSoftKeyboardOpen.ts';
import type {SuggestHandle} from '@/page/common/dto/terminal/SuggestHandle.ts';
import {attachMobileTerminalIme} from '@/page/common/util/mobileTerminalIme.ts';
import type {SuggestContext} from '@/page/common/dto/terminal/SuggestContext.ts';
import {TerminalAccessory, type TerminalAccessoryHandle} from './TerminalAccessory.tsx';
import {Ref, useCallback, useEffect, useImperativeHandle, useRef, useState} from 'react';
import {TerminalHandle, TerminalScreen} from '@/page/common/dto/terminal/TerminalHandle.ts';
import {useConfigService, useSystemUtilService, useTabManagerService, useTerminalBroadcastService} from '@/common/context/useAppHooks.tsx';
import {
    attachTerminalWebgl,
    bindTerminalHostPointer,
    closeTerminal,
    copyTerminalSelection,
    dispatchAccessoryData,
    parseOsc7Cwd,
    pasteClipboardToTerminal,
    pasteTerminalText,
    prepareMobileIme,
    readHardKeyboardConnected,
    readTerminalScreen,
    requestSoftInput,
    resolveTerminalOptions,
    resolveRootFontSizePx,
    resolveTerminalTheme,
    resolveTerminalWindowSize,
    stampXtermScreen,
    terminalIsPasting,
    type TerminalMenuPoint,
    waitForTerminalStableFit,
} from '@/page/common/util/terminal.ts';

export interface TerminalProps {
    settingKey: string;
    ref?: Ref<TerminalHandle>;
    suggest?: SuggestContext;
    closePane?: (dragMode: number, data: any) => void;
}

export function Terminal(props: TerminalProps) {
    const {t} = useI18n();
    const {paneId} = usePaneContext();
    const configService = useConfigService();
    const mobileViewport = usePlatformMobile();
    const cwdRef = useRef<string | null>(null);
    const layoutReadyRef = useRef<boolean>(false);
    const terminalRef = useRef<XTerm | null>(null);
    const mobileImeCleanupRef = useRef<ReturnType<typeof attachMobileTerminalIme> | null>(null);
    const systemUtilService = useSystemUtilService();
    const tabManagerService = useTabManagerService();
    const fitAddonRef = useRef<FitAddon | null>(null);
    const containerRef = useRef<HTMLDivElement>(null);
    const boxRef = useRef<HTMLDivElement | null>(null);
    const webglAddonRef = useRef<WebglAddon | null>(null);
    const {ref, settingKey, suggest, closePane} = props;
    const suggestRef = useRef<SuggestHandle | null>(null);
    const searchOpenRef = useRef<(() => void) | null>(null);
    const searchAddonRef = useRef<SearchAddon | null>(null);
    const nativeMobile = useDocumentClass('tm-native-mobile');
    const hardKeyboard = useDocumentClass('tm-hard-keyboard');
    const terminalHandleRef = useRef<TerminalHandle | null>(null);
    const terminalBroadcastService = useTerminalBroadcastService();
    const [settingsOpen, setSettingsOpen] = useState<boolean>(false);
    const [fontRevision, setFontRevision] = useState(0);
    const accessoryRef = useRef<TerminalAccessoryHandle | null>(null);
    const cleanupResizeObserverRef = useRef<(() => void) | null>(null);
    const [readyTerminal, setReadyTerminal] = useState<XTerm | null>(null);
    const onDataCallbacksRef = useRef<Set<(data: string) => void>>(new Set());
    const keyHandlersRef = useRef<Array<(event: KeyboardEvent) => boolean>>([]);
    const onResizeCallbackRef = useRef<((size: WindowSize) => void) | null>(null);
    const lastReportedSizeRef = useRef<{ cols: number; rows: number } | null>(null);
    const [backgroundHintDismissed, setBackgroundHintDismissed] = useState(() => configService.selectBackgroundHintDone());
    const [pasteMenuPoint, setPasteMenuPoint] = useState<TerminalMenuPoint | null>(null);

    const closeSettings = (): void => {
        setSettingsOpen(false);
    };

    const handleDismissBackgroundHint = (): void => {
        configService.saveBackgroundHintDone();
        setBackgroundHintDismissed(true);
    };

    const broadcastData = (data: string): void => {
        terminalBroadcastService.dispatch(paneId, data);
    };

    const captureCwd = (payload: string): boolean => {
        const cwd = parseOsc7Cwd(payload);
        if (cwd === null) {
            return false;
        }
        cwdRef.current = cwd;
        return true;
    };

    const dispatchKeyEvent = (event: KeyboardEvent): boolean => {
        for (const handler of keyHandlersRef.current) {
            if (!handler(event)) {
                return false;
            }
        }
        return true;
    };

    const registerKeyHandler = useCallback((handler: (event: KeyboardEvent) => boolean): (() => void) => {
        keyHandlersRef.current = [...keyHandlersRef.current, handler];
        return () => {
            keyHandlersRef.current = keyHandlersRef.current.filter(h => h !== handler);
        };
    }, []);

    const disposeWebglAddon = (): void => {
        const webgl = webglAddonRef.current;
        if (!webgl) {
            return;
        }
        webgl.dispose();
        webglAddonRef.current = null;
    };

    const refitTerminal = (): void => {
        const container = containerRef.current;
        const terminal = terminalRef.current;
        if (!container || !terminal) {
            return;
        }
        disposeWebglAddon();
        handleResize(container);
        loadWebglAddon(terminal);
    };

    const init = (onLayoutLoad: () => void): void => {
        const container = containerRef.current;
        if (!container) {
            return;
        }
        if (terminalRef.current) {
            if (layoutReadyRef.current) {
                onLayoutLoad();
            }
            return;
        }
        layoutReadyRef.current = false;
        const setting = configService.selectTerminalSetting(settingKey);
        const terminal = new XTerm({
            allowProposedApi: true,
            theme: resolveTerminalTheme(),
            ...resolveTerminalOptions(setting, mobileViewport),
        });
        terminalRef.current = terminal;
        const fitAddon = new FitAddon();
        fitAddonRef.current = fitAddon;

        const searchAddon = new SearchAddon();
        searchAddonRef.current = searchAddon;

        terminal.loadAddon(fitAddon);
        terminal.loadAddon(searchAddon);
        terminal.loadAddon(new Unicode11Addon());
        terminal.unicode.activeVersion = '11';
        terminal.attachCustomKeyEventHandler(dispatchKeyEvent);
        terminal.parser.registerOscHandler(7, captureCwd);
        terminal.onData(input);
        terminal.open(container);
        stampXtermScreen(container);
        if (mobileViewport) {
            prepareMobileIme(terminal, readHardKeyboardConnected());
            try {
                mobileImeCleanupRef.current = attachMobileTerminalIme(terminal);
            } catch (error) {
                console.warn(error);
            }
        }
        terminal.focus();
        if (mobileViewport && !readHardKeyboardConnected()) {
            requestSoftInput(true);
        }
        setReadyTerminal(terminal);

        waitForTerminalStableFit(
            () => fitAddonRef.current?.proposeDimensions(),
            dimsReady => {
                if (dimsReady) {
                    handleResize(container);
                }
                const terminal = terminalRef.current;
                if (terminal) {
                    loadWebglAddon(terminal);
                    terminal.focus();
                    if (mobileViewport && !readHardKeyboardConnected()) {
                        requestSoftInput(true);
                    }
                }
                layoutReadyRef.current = true;
                onLayoutLoad();
            },
        );
    };

    const setupResizeObserver = (container: HTMLDivElement): void => {
        const resizeObserver = new ResizeObserver(() => handleResize(container));
        resizeObserver.observe(container);
        cleanupResizeObserverRef.current = () => resizeObserver.disconnect();
    };

    const loadWebglAddon = (terminal: XTerm): void => {
        const webgl = attachTerminalWebgl(terminal, () => {
            if (webglAddonRef.current === webgl) {
                webglAddonRef.current = null;
            }
        });
        webglAddonRef.current = webgl;
    };

    const onWindowResize = (cb: (size: WindowSize) => void) => {
        onResizeCallbackRef.current = cb;
        return () => {
            if (onResizeCallbackRef.current === cb) {
                onResizeCallbackRef.current = null;
            }
        };
    };

    const handleResize = (container: HTMLDivElement) => {
        const fitAddon = fitAddonRef.current;
        const terminal = terminalRef.current;
        if (!fitAddon || !terminal) {
            return;
        }
        const proposed = fitAddon.proposeDimensions();
        if (!proposed || proposed.cols <= 0 || proposed.rows <= 0) {
            return;
        }
        fitAddon.fit();
        const size = resolveTerminalWindowSize(terminal, container);
        const last = lastReportedSizeRef.current;
        if (last && last.cols === size.cols && last.rows === size.rows) {
            return;
        }
        lastReportedSizeRef.current = {cols: size.cols, rows: size.rows};
        onResizeCallbackRef.current?.(size);
    };

    const write = (data: string | Uint8Array) => {
        terminalRef.current?.write(data);
    };

    const onData = (cb: (data: string) => void) => {
        onDataCallbacksRef.current.add(cb);
        return () => {
            onDataCallbacksRef.current.delete(cb);
        };
    };

    const rawInput = (data: string): void => {
        try {
            mobileImeCleanupRef.current?.externalInput();
        } catch (error) {
            console.warn(error);
        }
        for (const cb of onDataCallbacksRef.current) {
            cb(data);
        }
    };

    const input = (data: string): void => {
        try {
            const next = terminalRef.current && terminalIsPasting(terminalRef.current) ? data : accessoryRef.current ? accessoryRef.current.consume(data) : data;
            if (!dispatchAccessoryData(next, dispatchKeyEvent)) {
                return;
            }
            broadcastData(next);
            rawInput(next);
        } catch (error) {
            console.warn(error);
        }
    };

    const paste = (text: string): void => {
        const terminal = terminalRef.current;
        if (!terminal) {
            return;
        }
        pasteTerminalText(terminal, text);
        mobileImeCleanupRef.current?.externalInput();
    };

    const handleOpenSearch = (): void => {
        const open = searchOpenRef.current;
        if (!open) {
            throw new Error('TerminalSearch open unavailable');
        }
        open();
    };

    const handleOpenSuggest = (): void => {
        const suggest = suggestRef.current;
        if (!suggest) {
            throw new Error('CommandSuggest openPanel unavailable');
        }
        suggest.openPanel();
    };

    const handleHideKeyboard = (): void => {
        const terminal = terminalRef.current;
        if (!terminal) {
            throw new Error('terminal unavailable');
        }
        terminal.textarea?.setAttribute('inputmode', 'none');
        terminal.blur();
        requestSoftInput(false);
    };

    const getCwd = (): string | null => {
        return cwdRef.current;
    };

    const getScreen = (): TerminalScreen => {
        const terminal = terminalRef.current;
        if (!terminal) {
            throw new Error('Terminal is not initialized');
        }
        return readTerminalScreen(terminal);
    };

    const getWindowSize: () => WindowSize = () => {
        const terminal = terminalRef.current;
        const container = containerRef.current;
        if (!terminal || !container) {
            throw new Error('Terminal is not initialized');
        }
        return resolveTerminalWindowSize(terminal, container);
    };

    const getSearchAddon = useCallback((): SearchAddon | null => {
        return searchAddonRef.current;
    }, []);

    const toggleSettingsPanel = (): void => {
        setSettingsOpen(prev => !prev);
    };

    const dispose = (): void => {
        setPasteMenuPoint(null);
        mobileImeCleanupRef.current?.dispose();
        mobileImeCleanupRef.current = null;
        webglAddonRef.current?.dispose();
        webglAddonRef.current = null;
        terminalRef.current?.dispose();
        terminalRef.current = null;
        fitAddonRef.current = null;
        searchAddonRef.current = null;
        lastReportedSizeRef.current = null;
        layoutReadyRef.current = false;
        onDataCallbacksRef.current.clear();
        setReadyTerminal(null);
    };

    useBackDismiss(settingsOpen, closeSettings, `terminal-settings-${paneId}`);

    useImperativeHandle(ref, () => {
        const handle: TerminalHandle = {init, onWindowResize, write, input, onData, getCwd, getScreen, getWindowSize, dispose, toggleSettingsPanel};
        terminalHandleRef.current = handle;
        return handle;
    });

    useEffect(() => {
        const container = containerRef.current;
        if (!container) {
            return;
        }
        setupResizeObserver(container);
        return () => {
            cleanupResizeObserverRef.current?.();
            cleanupResizeObserverRef.current = null;
        };
    }, [paneId]);

    useEffect(() => {
        const unsubscribe = tabManagerService.addCloseGuardListener((dragMode: number, data: any) => {
            closeTerminal(paneId, dragMode, data, closePane);
            return true;
        });
        return () => {
            unsubscribe();
        };
    }, [paneId, tabManagerService]);

    useEffect(() => {
        if (!readyTerminal) {
            return;
        }
        const pane = tabManagerService.getPane(paneId);
        if (!pane) {
            return;
        }
        return terminalBroadcastService.register(paneId, pane.tabBar.title, rawInput);
    }, [paneId, readyTerminal, tabManagerService, terminalBroadcastService]);

    useEffect(() => {
        if (!mobileViewport || !readyTerminal) {
            return;
        }
        prepareMobileIme(readyTerminal, hardKeyboard);
        if (hardKeyboard) {
            requestSoftInput(false);
        }
    }, [mobileViewport, readyTerminal, hardKeyboard]);

    useEffect(() => {
        const host = containerRef.current;
        if (host === null) {
            return;
        }
        return bindTerminalHostPointer(
            host,
            () => terminalRef.current,
            mobileViewport,
            () => {
                pasteClipboardToTerminal(() => systemUtilService.readClipboardText(), paste);
            },
            terminal => {
                copyTerminalSelection(terminal, text => systemUtilService.writeClipboardText(text));
            },
            setPasteMenuPoint,
            (size, finished) => {
                const host = containerRef.current;
                if (host) {
                    handleResize(host);
                }
                if (finished) {
                    const setting = configService.selectTerminalSetting(settingKey);
                    configService.saveTerminalSetting(settingKey, {...setting, fontSize: size / resolveRootFontSizePx()});
                    setFontRevision(value => value + 1);
                }
            },
        );
    }, [mobileViewport, readyTerminal, systemUtilService]);

    useEffect(() => {
        const host = containerRef.current;
        if (!readyTerminal || !host) {
            return;
        }
        const onPaste = (event: ClipboardEvent): void => {
            if (!event.clipboardData) {
                return;
            }
            event.preventDefault();
            event.stopImmediatePropagation();
            paste(event.clipboardData.getData('text/plain'));
        };
        host.addEventListener('paste', onPaste, true);
        return () => host.removeEventListener('paste', onPaste, true);
    }, [readyTerminal]);

    return (
        <Box className={mobileViewport ? 'tm-terminal-shell is-mobile' : 'tm-terminal-shell'}>
            {nativeMobile && !backgroundHintDismissed ? (
                <Box className="tm-terminal-background-hint">
                    <Typography className="tm-agent-page-empty-message">{t('protocol.session.backgroundHint')}</Typography>
                    <ButtonBase disableRipple className="tm-terminal-background-hint-close" aria-label={t('common.close')} onClick={handleDismissBackgroundHint}>
                        <Close/>
                    </ButtonBase>
                </Box>
            ) : null}
            <Box ref={boxRef} className={'tm-terminal-box'}>
                <Box ref={containerRef} className={'tm-terminal-host'} data-tm-gesture-ignore="true" data-tm-terminal="true"></Box>
                {readyTerminal && <TerminalDrop hostRef={boxRef} getCwd={getCwd} onInput={paste}/>}
                {readyTerminal && <TerminalShortcuts terminal={readyTerminal} onPaste={paste} registerKeyHandler={registerKeyHandler}/>}
                {readyTerminal && mobileViewport && <TerminalSelectBar terminal={readyTerminal} hostRef={containerRef} onPaste={paste} pasteMenuPoint={pasteMenuPoint} onPasteMenuChange={setPasteMenuPoint}/>}
                {readyTerminal && <TerminalSearch terminal={readyTerminal} getSearchAddon={getSearchAddon} openRef={searchOpenRef} registerKeyHandler={registerKeyHandler}/>}
                {readyTerminal && <TerminalSnippet terminal={readyTerminal} onInput={input} registerKeyHandler={registerKeyHandler}/>}
                {readyTerminal && <TerminalBroadcast paneId={paneId} registerKeyHandler={registerKeyHandler}/>}
                {suggest && readyTerminal && <CommandSuggest ref={suggestRef} terminal={readyTerminal} hostRef={boxRef} context={suggest} onInput={input} registerKeyHandler={registerKeyHandler}/>}
                {mobileViewport && nativeMobile ? <TerminalAccessory ref={accessoryRef} onHide={handleHideKeyboard} onInput={input} onOpenSearch={handleOpenSearch} onOpenSuggest={handleOpenSuggest}/> : null}
            </Box>
            <Box className={settingsOpen ? 'tm-terminal-settings-drawer is-open' : 'tm-terminal-settings-drawer'}>
                <TerminalSettingsPanel terminal={readyTerminal} settingKey={settingKey} hostRef={boxRef} onInput={input} onClose={toggleSettingsPanel} onRefit={refitTerminal} fontRevision={fontRevision}/>
            </Box>
        </Box>
    );
}
