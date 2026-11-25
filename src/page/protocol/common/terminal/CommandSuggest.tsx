import {Box} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import type {Terminal as XTerm} from '@xterm/xterm';
import {CommandSuggestPanel} from './CommandSuggestPanel.tsx';
import {usePlatformMobile} from '@/common/context/usePlatformMobile';
import {TERMINAL_KIND} from '@/common/model/req/ReqCommandSuggest.ts';
import {AlertDialog} from '@/common/components/dialog/AlertDialog.tsx';
import {ConfirmDialog} from '@/common/components/dialog/ConfirmDialog.tsx';
import type {SuggestHandle} from '@/page/common/dto/terminal/SuggestHandle.ts';
import type {SuggestContext} from '@/page/common/dto/terminal/SuggestContext.ts';
import {Ref, RefObject, useEffect, useImperativeHandle, useRef, useState} from 'react';
import type {GhostHandle, SuggestAnchor} from '@/page/common/util/commandSuggestUtil.ts';
import {BizError, RESULT_CODE, resolveResultMessage} from '@/common/model/resp/ResultModel.ts';
import {SUGGEST_COUNT_DEFAULT, SUGGEST_COUNT_MAX, SUGGEST_COUNT_MIN} from '@/common/model/Config.ts';
import {useAccountService, useAiShareConsentService, useCommandSuggestService, useConfigService} from '@/common/context/useAppHooks.tsx';
import {createGhost, isAltScreen, isMouseCsi, isVimInsert, mapSuggestList, readCurrentLine, readRecentOutput, resolveAcceptInput, resolveAnchor, resolveDebounceMs, resolvePanelHeight, resolveTypedPreview} from '@/page/common/util/commandSuggestUtil.ts';

const PANEL_WIDTH = 260;
const PANEL_MAX_WIDTH = 360;
const HISTORY_MAX_ITEMS = 12;
const GHOST_SUGGEST_COUNT = 1;
const MOBILE_PANEL_ANCHOR: SuggestAnchor = {left: 0, top: 0};

const IGNORE_DATA = new Set(['\u001b[I', '\u001b[O']);
const RESET_DATA = new Set(['\r', '\n', '\u0003', '\u0004', '\u000c', '\u0015', '\u001a', '\u001b']);

interface SuggestState {
    asked: string;
    list: string[];
    typed: string;
    error: string;
    selected: number;
    loading: boolean;
    panelOpen: boolean;
    instruction: string;
}

const EMPTY_STATE: SuggestState = {list: [], typed: '', error: '', selected: 0, loading: false, panelOpen: false, instruction: '', asked: ''};

export interface CommandSuggestProps {
    terminal: XTerm;
    context: SuggestContext;
    ref?: Ref<SuggestHandle>;
    onInput: (data: string) => void;
    hostRef: RefObject<HTMLElement | null>;
    registerKeyHandler: (handler: (event: KeyboardEvent) => boolean) => () => void;
}

export function CommandSuggest(props: CommandSuggestProps) {
    const {t} = useI18n();
    const ikiRef = useRef<number[]>([]);
    const lastLineRef = useRef<string>('');
    const lastDataAtRef = useRef<number>(0);
    const historyRef = useRef<string[]>([]);
    const configService = useConfigService();
    const accountService = useAccountService();
    const enabledRef = useRef<boolean>(false);
    const mobileViewport = usePlatformMobile();
    const timerRef = useRef<number | null>(null);
    const suggestService = useCommandSuggestService();
    const ghostRef = useRef<GhostHandle | null>(null);
    const stateRef = useRef<SuggestState>(EMPTY_STATE);
    const abortRef = useRef<AbortController | null>(null);
    const [shareAsking, setShareAsking] = useState(false);
    const aiShareConsentService = useAiShareConsentService();
    const contextRef = useRef<SuggestContext>(props.context);
    const sharePendingRef = useRef<(() => void) | null>(null);
    const panelCountRef = useRef<number>(SUGGEST_COUNT_DEFAULT);
    const [state, setState] = useState<SuggestState>(EMPTY_STATE);
    const [anchor, setAnchor] = useState<SuggestAnchor | null>(null);
    const [loginAlert, setLoginAlert] = useState<string | null>(null);
    const {terminal, hostRef, context, onInput, ref, registerKeyHandler} = props;

    contextRef.current = context;

    const update = (patch: Partial<SuggestState>): void => {
        const next = {...stateRef.current, ...patch};
        stateRef.current = next;
        setState(next);
    };

    const clearSuggest = (): void => {
        if (timerRef.current !== null) {
            window.clearTimeout(timerRef.current);
            timerRef.current = null;
        }
        abortRef.current?.abort();
        abortRef.current = null;
        ghostRef.current?.clear();
        update(EMPTY_STATE);
    };

    const resolveSuggestCount = (openPanel: boolean): number => {
        if (!openPanel) {
            return GHOST_SUGGEST_COUNT;
        }
        return Math.min(SUGGEST_COUNT_MAX, Math.max(SUGGEST_COUNT_MIN, panelCountRef.current));
    };

    const loadHistory = async (): Promise<string[]> => {
        const current = contextRef.current;
        if (current.terminalKind === TERMINAL_KIND.SSH) {
            return suggestService.loadSshHistory(current.deviceId);
        }
        if (current.terminalKind === TERMINAL_KIND.SHELL && current.shellId) {
            return suggestService.loadLocalHistory(current.shellId);
        }
        return [];
    };

    const requestSuggest = async (openPanel: boolean, instruction: string): Promise<void> => {
        if (!enabledRef.current || accountService.getAccount() === null || !aiShareConsentService.selectConsentDone()) {
            return;
        }
        if (isAltScreen(terminal) && !isVimInsert(terminal)) {
            return;
        }
        const currentLine = readCurrentLine(terminal);
        if (!openPanel && currentLine === lastLineRef.current) {
            return;
        }
        lastLineRef.current = currentLine;
        abortRef.current?.abort();
        const controller = new AbortController();
        abortRef.current = controller;
        update({
            list: [],
            error: '',
            selected: 0,
            loading: true,
            asked: instruction,
            panelOpen: openPanel,
            typed: resolveTypedPreview(currentLine),
        });
        try {
            if (historyRef.current.length === 0) {
                historyRef.current = await loadHistory();
            }
            if (controller.signal.aborted) {
                return;
            }
            const list = mapSuggestList(
                currentLine,
                await suggestService.suggest(
                    {
                        instruction,
                        currentLine,
                        history: historyRef.current.filter(line => line.trim().length > 0).slice(-HISTORY_MAX_ITEMS),
                        deviceId: contextRef.current.deviceId,
                        shellKind: contextRef.current.shellKind,
                        terminalKind: contextRef.current.terminalKind,
                        recentOutput: readRecentOutput(terminal),
                        suggestCount: resolveSuggestCount(openPanel),
                        altScreen: isAltScreen(terminal),
                    },
                    controller.signal,
                    items => {
                        if (controller.signal.aborted) {
                            return;
                        }
                        update({list: mapSuggestList(currentLine, items), error: '', selected: 0, loading: true});
                    },
                ),
            );
            if (controller.signal.aborted) {
                return;
            }
            update({list, error: '', selected: 0, loading: false});
        } catch (err) {
            if (controller.signal.aborted) {
                return;
            }
            if (openPanel && err instanceof BizError && err.code === RESULT_CODE.AI_MODEL_NOT_FOUND) {
                update({list: [], error: resolveResultMessage(err.code), selected: 0, loading: false});
                return;
            }
            update({list: [], error: '', selected: 0, loading: false});
        } finally {
            if (abortRef.current === controller) {
                abortRef.current = null;
            }
        }
    };

    const acceptSuggest = (text: string): void => {
        if (!text) {
            return;
        }
        const payload = resolveAcceptInput(text, isAltScreen(terminal), terminal.modes.bracketedPasteMode);
        if (!payload) {
            return;
        }
        terminal.focus();
        onInput(payload);
        clearSuggest();
    };

    const closePanel = (): void => {
        clearSuggest();
        terminal.focus();
    };

    const openPanel = (): void => {
        if (accountService.getAccount() === null) {
            setLoginAlert(t('account.login.fullFeaturesRequired'));
            return;
        }
        if (!aiShareConsentService.selectConsentDone()) {
            sharePendingRef.current = openPanel;
            setShareAsking(true);
            return;
        }
        if (isAltScreen(terminal) && !isVimInsert(terminal)) {
            return;
        }
        if (!enabledRef.current) {
            return;
        }
        if (stateRef.current.list.length > 0) {
            update({panelOpen: true});
            return;
        }
        requestSuggest(true, '').then();
    };

    const acceptSelected = (): void => {
        const {list, selected} = stateRef.current;
        acceptSuggest(list[selected] ?? '');
    };

    const moveSelected = (delta: number): void => {
        const {list, selected} = stateRef.current;
        if (list.length === 0) {
            return;
        }
        update({selected: (selected + delta + list.length) % list.length});
    };

    const submitInstruction = (): void => {
        const {instruction, asked} = stateRef.current;
        const value = instruction.trim();
        if (!value || value === asked) {
            acceptSelected();
            return;
        }
        requestSuggest(true, value).then();
    };

    useImperativeHandle(ref, () => ({openPanel}));

    useEffect(() => {
        const host = hostRef.current;
        if (!host) {
            console.warn('suggest host missing');
            return;
        }
        ghostRef.current = createGhost(terminal, host);
        return () => {
            ghostRef.current?.clear();
            ghostRef.current = null;
        };
    }, [terminal, hostRef]);

    useEffect(() => {
        const disposable = terminal.onWriteParsed(() => {
            const current = stateRef.current.list[stateRef.current.selected];
            if (!current) {
                ghostRef.current?.clear();
                return;
            }
            ghostRef.current?.render(current);
        });
        return () => {
            disposable.dispose();
        };
    }, [terminal]);

    useEffect(() => {
        let alive = true;
        configService
            .selectConfig()
            .then(config => {
                if (!alive) {
                    return;
                }
                enabledRef.current = Boolean(config?.suggest_enabled);
                const count = config?.suggest_count ?? SUGGEST_COUNT_DEFAULT;
                panelCountRef.current = Math.min(SUGGEST_COUNT_MAX, Math.max(SUGGEST_COUNT_MIN, count));
            })
            .catch(() => {
                enabledRef.current = false;
            });
        return () => {
            alive = false;
        };
    }, [configService]);

    useEffect(() => {
        historyRef.current = [];
        const isSsh = context.terminalKind === TERMINAL_KIND.SSH;
        const isLocalShell = context.terminalKind === TERMINAL_KIND.SHELL && !!context.shellId;
        if (!isSsh && !isLocalShell) {
            return;
        }
        if (isSsh && context.sessionOpened === false) {
            return;
        }
        let alive = true;
        loadHistory()
            .then(history => {
                if (alive) {
                    historyRef.current = history;
                }
            })
            .catch(() => {
                historyRef.current = [];
            });
        return () => {
            alive = false;
        };
    }, [context.deviceId, context.terminalKind, context.shellId, context.sessionOpened, suggestService]);

    useEffect(() => {
        const dataDisposable = terminal.onData(data => {
            if (!enabledRef.current || IGNORE_DATA.has(data) || isMouseCsi(data)) {
                return;
            }
            clearSuggest();
            if (RESET_DATA.has(data) || (isAltScreen(terminal) && !isVimInsert(terminal))) {
                ikiRef.current = [];
                lastLineRef.current = '';
                lastDataAtRef.current = 0;
                return;
            }
            const now = Date.now();
            if (lastDataAtRef.current > 0) {
                const dt = now - lastDataAtRef.current;
                if (dt > 0 && dt < 2000) {
                    ikiRef.current = [...ikiRef.current, dt].slice(-8);
                }
            }
            lastDataAtRef.current = now;
            timerRef.current = window.setTimeout(() => {
                timerRef.current = null;
                requestSuggest(false, '').then();
            }, resolveDebounceMs(ikiRef.current));
        });
        return () => {
            dataDisposable.dispose();
        };
    }, [terminal]);

    useEffect(() => {
        return registerKeyHandler(event => {
            if (event.type !== 'keydown') {
                return true;
            }
            if (event.altKey && (event.key === '/' || event.code === 'Slash' || event.code === 'NumpadDivide')) {
                openPanel();
                return false;
            }
            if (!enabledRef.current) {
                return true;
            }
            const {list, panelOpen} = stateRef.current;
            if (event.key === 'Escape') {
                if (!panelOpen && list.length === 0) {
                    return true;
                }
                closePanel();
                return false;
            }
            if (panelOpen && (event.key === 'ArrowDown' || event.key === 'ArrowUp')) {
                moveSelected(event.key === 'ArrowDown' ? 1 : -1);
                return false;
            }
            if (list.length === 0) {
                return true;
            }
            if (event.key === 'Tab') {
                acceptSelected();
                return false;
            }
            if (panelOpen && event.key === 'Enter') {
                acceptSelected();
                return false;
            }
            return true;
        });
    }, [terminal, registerKeyHandler]);

    useEffect(() => {
        if (!state.panelOpen) {
            return;
        }
        const onKeyDown = (event: KeyboardEvent): void => {
            if (event.key !== 'Escape') {
                return;
            }
            event.preventDefault();
            closePanel();
        };
        document.addEventListener('keydown', onKeyDown, true);
        return () => {
            document.removeEventListener('keydown', onKeyDown, true);
        };
    }, [state.panelOpen]);

    useEffect(() => {
        const ghost = ghostRef.current;
        if (!ghost) {
            return;
        }
        const current = state.list[state.selected];
        if (!current) {
            ghost.clear();
            return;
        }
        ghost.render(current);
    }, [state.list, state.selected]);

    useEffect(() => {
        if (!state.panelOpen || mobileViewport) {
            setAnchor(null);
            return;
        }
        const host = hostRef.current;
        if (!host) {
            return;
        }
        const panelWidth = state.list.some(item => item.includes('\n')) ? PANEL_MAX_WIDTH : PANEL_WIDTH;
        setAnchor(resolveAnchor(terminal, host, panelWidth, resolvePanelHeight(state.list, state.loading)));
    }, [state.panelOpen, state.list, state.loading, terminal, hostRef, mobileViewport]);

    const panelWidth = state.list.some(item => item.includes('\n')) ? PANEL_MAX_WIDTH : PANEL_WIDTH;
    const panelAnchor = mobileViewport ? MOBILE_PANEL_ANCHOR : anchor;

    return (
        <>
            <AlertDialog duration={4000} open={loginAlert != null} handleClose={() => setLoginAlert(null)} message={loginAlert} type="error"/>
            <ConfirmDialog
                open={shareAsking}
                title={t('agent.input.aiShareTitle')}
                message={t('agent.input.aiShareMessage')}
                onCancel={() => {
                    sharePendingRef.current = null;
                    setShareAsking(false);
                }}
                onConfirm={() => {
                    const run = sharePendingRef.current;
                    sharePendingRef.current = null;
                    setShareAsking(false);
                    aiShareConsentService.saveConsentDone();
                    if (run) {
                        run();
                    }
                }}
            >
                <Box sx={{mt: 1, fontSize: 13, lineHeight: 1.6, color: 'var(--tm-text-secondary)'}}>{t('agent.input.aiShareDetail')}</Box>
            </ConfirmDialog>
            {state.panelOpen && panelAnchor ? (
                <CommandSuggestPanel
                    mobile={mobileViewport}
                    anchor={panelAnchor}
                    width={panelWidth}
                    maxWidth={PANEL_MAX_WIDTH}
                    onClose={closePanel}
                    onMove={moveSelected}
                    onAccept={acceptSelected}
                    onSubmit={submitInstruction}
                    list={state.list}
                    typed={state.typed}
                    error={state.error}
                    loading={state.loading}
                    selected={state.selected}
                    instruction={state.instruction}
                    onInstructionChange={value => update({instruction: value})}
                    onHover={index => update({selected: index})}
                    onPick={index => acceptSuggest(state.list[index])}
                />
            ) : null}
        </>
    );
}
