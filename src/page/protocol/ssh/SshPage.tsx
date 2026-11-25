import {Box} from "@mui/material";
import {PROTOCOL} from "@/common/const/Const.ts";
import {usePaneContext} from "@/common/components/tab";
import {ReqDevice} from "@/common/model/req/ReqDevice.ts";
import {AuthPage} from "@/page/protocol/ssh/auth/AuthPage.tsx";
import {SshOpenEvent} from "@/common/model/req/event/WsSsh.ts";
import {StepPage} from "@/page/protocol/common/step/StepPage.tsx";
import {TERMINAL_KIND} from "@/common/model/req/ReqCommandSuggest.ts";
import {Terminal} from "@/page/protocol/common/terminal/Terminal.tsx";
import {useCallback, useEffect, useMemo, useRef, useState} from "react";
import {resolveTerminalSettingKey} from "@/page/common/util/terminal.ts";
import {TerminalHandle} from "@/page/common/dto/terminal/TerminalHandle.ts";
import {DeviceTabBarSync} from "@/page/protocol/common/DeviceTabBarSync.tsx";
import {useSshService, useTabManagerService} from "@/common/context/useAppHooks.tsx";
import {createTerminalSettingsAction} from "@/page/protocol/common/terminal/TerminalSettingsAction.tsx";

export interface SshProps {
    reqDevice: ReqDevice
}

export function SshPage(props: SshProps) {
    const {reqDevice} = props;
    const {paneId} = usePaneContext();
    const sshService = useSshService()
    const openedRef = useRef<boolean>(false);
    const tabManagerService = useTabManagerService();
    const terminalRef = useRef<TerminalHandle>(null);
    const [opened, setOpened] = useState<boolean>(false);
    const sshOpenRef = useRef<(() => void) | null>(null);
    const restoreCwdRef = useRef<string | null>(null);
    const toggleSettings = useCallback(() => terminalRef.current?.toggleSettingsPanel(), []);
    const settingsAction = useMemo(() => createTerminalSettingsAction(toggleSettings), [toggleSettings]);

    const init = async () => {
        if (!terminalRef.current) {
            return
        }
        terminalRef.current.init(() => {
            doConnect(false).then()
        })
    }

    const doConnect = async (force: boolean) => {
        if (openedRef.current && !force) {
            return
        }
        const terminal = terminalRef.current;
        if (!terminal) {
            return
        }
        openedRef.current = true;
        destroySsh()
        restoreCwdRef.current = force ? terminal.getCwd() : null;
        const size = terminal.getWindowSize();
        const payload: SshOpenEvent = {data: {deviceId: reqDevice.device.id, size: size}}
        sshOpenRef.current = await sshService.handleConnect(paneId, payload, terminal, setOpened)
    }

    const destroyAll = (): void => {
        destroySsh()
        terminalRef.current?.dispose()
    }

    const destroySsh = (): void => {
        sshOpenRef.current?.();
        sshOpenRef.current = null;
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
        const unsubscribe = sshService.addConnectRetryListener(paneId, () => {
            doConnect(true).then()
        });
        return () => {
            unsubscribe();
        };
    }, [paneId, sshService]);

    useEffect(() => {
        if (!opened) {
            return;
        }
        const cwd = restoreCwdRef.current;
        restoreCwdRef.current = null;
        if (!cwd) {
            return;
        }
        terminalRef.current?.input(`cd '${cwd.split("'").join("'\\''")}'\r`);
    }, [opened]);

    return (
        <Box sx={{position: 'relative', width: '100%', height: '100%'}}>
            <DeviceTabBarSync deviceId={reqDevice.device.id} protocol={PROTOCOL.SSH} {...settingsAction}/>
            <Terminal ref={terminalRef} settingKey={resolveTerminalSettingKey(reqDevice.device.id, TERMINAL_KIND.SSH)} suggest={{deviceId: reqDevice.device.id, terminalKind: TERMINAL_KIND.SSH, sessionOpened: opened}} closePane={destroyAll}></Terminal>
            <StepPage protocol={PROTOCOL.SSH} connectService={sshService} reqDevice={reqDevice} sessionId={paneId} authPage={<AuthPage connectService={sshService} reqDevice={reqDevice} sessionId={paneId}/>}/>
        </Box>
    );
}
