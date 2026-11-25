import {Box} from "@mui/material";
import {ShellDescr} from "@/common/model/ShellDescr.ts";
import {useCallback, useEffect, useRef, useState} from "react";
import {ShellOpenEvent} from "@/common/model/req/event/WsShell.ts";
import {TERMINAL_KIND} from "@/common/model/req/ReqCommandSuggest.ts";
import {Terminal} from "@/page/protocol/common/terminal/Terminal.tsx";
import {resolveShellPromptKind} from "@/page/common/util/shellUtil.ts";
import {ConnectLost} from "@/page/protocol/common/step/ConnectLost.tsx";
import {PANE_DRAG_MODE} from "@/common/components/tab/model/PaneDrag.ts";
import {TerminalHandle} from "@/page/common/dto/terminal/TerminalHandle.ts";
import {PaneConfig, TabConfig, usePaneContext} from "@/common/components/tab";
import {markPaneTitleLost, resolveTerminalSettingKey} from "@/page/common/util/terminal.ts";
import {createTerminalSettingsAction} from "@/page/protocol/common/terminal/TerminalSettingsAction.tsx";
import {useDeviceService, useShellService, useTabManagerService} from "@/common/context/useAppHooks.tsx";

export interface ShellPageProps {
    cwd?: string;
    shellDescr: ShellDescr;
    attachProcessId?: string;
}

export function ShellPage(props: ShellPageProps) {
    const {paneId} = usePaneContext();
    const shellService = useShellService();
    const deviceService = useDeviceService();
    const openedRef = useRef<boolean>(false);
    const {cwd, shellDescr, attachProcessId} = props;
    const tabManagerService = useTabManagerService();
    const terminalRef = useRef<TerminalHandle>(null);
    const [lost, setLost] = useState<boolean>(false);
    const [opened, setOpened] = useState<boolean>(false);
    const shellOpenRef = useRef<(() => void) | null>(null);
    const attachProcessIdRef = useRef<string | undefined>(attachProcessId);
    const [localDeviceId, setLocalDeviceId] = useState<string | null>(null);
    const toggleSettings = useCallback(() => terminalRef.current?.toggleSettingsPanel(), []);

    const init = async () => {
        if (!terminalRef.current) {
            return
        }
        terminalRef.current.init(() => {
            doConnect().then()
        })
    }

    const doConnect = async () => {
        if (openedRef.current) {
            return
        }
        const terminal = terminalRef.current;
        if (!terminal) {
            return
        }
        openedRef.current = true;
        const payload: ShellOpenEvent = {data: {size: terminal.getWindowSize(), shellId: shellDescr.shell_id, attachProcessId: attachProcessIdRef.current, cwd}}
        shellOpenRef.current = await shellService.handleConnect(paneId, payload, terminal, setOpened)
    }

    const destroyShell = (): void => {
        shellOpenRef.current?.();
        shellOpenRef.current = null;
    }

    const handleReconnect = (): void => {
        destroyShell();
        openedRef.current = false;
        attachProcessIdRef.current = undefined;
        setLost(false);
        doConnect().then();
    }

    const closeShell = (dragMode: number, data: any): void => {
        if (dragMode == PANE_DRAG_MODE.TAB) {
            const tab = data as TabConfig;
            for (const pane of tab.panes) {
                if (pane.id === paneId) {
                    destroyShell();
                    break;
                }
            }
        } else {
            const pane = data as PaneConfig;
            if (pane.id != paneId) {
                return
            }
            destroyShell();
        }
    }

    useEffect(() => {
        const unsubscribe = tabManagerService.addPaneOpenListener(pane => {
            if (paneId !== pane.id) {
                return;
            }
            init().then();
        });
        return () => {
            unsubscribe();
        };
    }, [paneId, tabManagerService]);

    useEffect(() => {
        return shellService.addCloseListener(paneId, () => {
            setLost(true);
        });
    }, [paneId, shellService]);

    useEffect(() => {
        if (opened) {
            setLost(false);
        }
    }, [opened]);

    useEffect(() => {
        markPaneTitleLost(tabManagerService, paneId, lost);
    }, [lost, paneId, tabManagerService]);

    useEffect(() => {
        deviceService.localDevice().then(resp => {
            if (!resp) {
                setLocalDeviceId(null);
                return;
            }
            setLocalDeviceId(resp.device.id);
        }).catch(() => {
            setLocalDeviceId(null);
        });
    }, [deviceService]);

    useEffect(() => {
        const pane = tabManagerService.getPane(paneId);
        if (!pane) {
            return;
        }
        tabManagerService.updatePaneBar(paneId, {
            ...pane.tabBar,
            ...createTerminalSettingsAction(toggleSettings),
        });
    }, [paneId, tabManagerService, toggleSettings]);

    return (
        <Box sx={{position: 'relative', width: '100%', height: '100%'}}>
            <Terminal
                ref={terminalRef}
                closePane={closeShell}
                settingKey={resolveTerminalSettingKey(shellDescr.shell_id, TERMINAL_KIND.SHELL)}
                suggest={localDeviceId ? {
                    deviceId: localDeviceId,
                    shellId: shellDescr.shell_id,
                    terminalKind: TERMINAL_KIND.SHELL,
                    shellKind: resolveShellPromptKind(shellDescr.shell_id),
                } : undefined}
            />
            {lost ? <ConnectLost attempt={0} maxAttempt={0} countdown={0} onRetry={handleReconnect}/> : null}
        </Box>
    )
}