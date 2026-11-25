import {Box} from '@mui/material';
import {PROTOCOL} from "@/common/const/Const.ts";
import {usePaneContext} from "@/common/components/tab";
import {ReqDevice} from "@/common/model/req/ReqDevice.ts";
import {StepPage} from "@/page/protocol/common/step/StepPage.tsx";
import {TelnetOpenEvent} from "@/common/model/req/event/WsTelnet.ts";
import {TERMINAL_KIND} from "@/common/model/req/ReqCommandSuggest.ts";
import {Terminal} from "@/page/protocol/common/terminal/Terminal.tsx";
import {resolveTerminalSettingKey} from "@/page/common/util/terminal.ts";
import {TerminalHandle} from "@/page/common/dto/terminal/TerminalHandle.ts";
import {DeviceTabBarSync} from "@/page/protocol/common/DeviceTabBarSync.tsx";
import {ReactNode, useCallback, useEffect, useMemo, useRef, useState} from 'react';
import {useTabManagerService, useTelnetService} from "@/common/context/useAppHooks.tsx";
import {createTerminalSettingsAction} from "@/page/protocol/common/terminal/TerminalSettingsAction.tsx";

export interface TelnetPageProps {
    reqDevice: ReqDevice
}

export function TelnetPage(props: TelnetPageProps): ReactNode {
    const {reqDevice} = props;
    const {paneId} = usePaneContext();
    const openedRef = useRef<boolean>(false);
    const telnetService = useTelnetService();
    const [, setOpened] = useState<boolean>(false);
    const tabManagerService = useTabManagerService();
    const terminalRef = useRef<TerminalHandle>(null);
    const telnetOpenRef = useRef<(() => void) | null>(null);
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
        destroyTelnet()
        const size = terminal.getWindowSize();
        const payload: TelnetOpenEvent = {data: {deviceId: reqDevice.device.id, size: size}}
        telnetOpenRef.current = await telnetService.handleConnect(paneId, payload, terminal, setOpened)
    }

    const destroyAll = (): void => {
        destroyTelnet()
        terminalRef.current?.dispose()
    }

    const destroyTelnet = (): void => {
        telnetOpenRef.current?.();
        telnetOpenRef.current = null;
    }

    useEffect(() => {
        const unsubscribe = tabManagerService.addPaneOpenListener(pane => {
            if (paneId !== pane.id) {
                return;
            }
            init().then()
        });
        return () => {
            unsubscribe();
        };
    }, [paneId, tabManagerService]);

    useEffect(() => {
        const unsubscribe = telnetService.addConnectRetryListener(paneId, () => {
            doConnect(true).then()
        });
        return () => {
            unsubscribe();
        };
    }, [paneId, telnetService]);

    return (
        <Box sx={{position: 'relative', width: '100%', height: '100%'}}>
            <DeviceTabBarSync deviceId={reqDevice.device.id} protocol={PROTOCOL.TELNET} {...settingsAction}/>
            <Terminal ref={terminalRef} settingKey={resolveTerminalSettingKey(reqDevice.device.id, TERMINAL_KIND.TELNET)} suggest={{deviceId: reqDevice.device.id, terminalKind: TERMINAL_KIND.TELNET}} closePane={destroyAll}></Terminal>
            <StepPage protocol={PROTOCOL.TELNET} connectService={telnetService} reqDevice={reqDevice} sessionId={paneId}/>
        </Box>
    );
}


