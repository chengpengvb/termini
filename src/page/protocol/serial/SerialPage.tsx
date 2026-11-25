import {Box} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {usePaneContext} from "@/common/components/tab";
import {SerialOpenEvent} from "@/common/model/req/event/WsSerial.ts";
import {TERMINAL_KIND} from "@/common/model/req/ReqCommandSuggest.ts";
import {Terminal} from "@/page/protocol/common/terminal/Terminal.tsx";
import {resolveTerminalSettingKey} from "@/page/common/util/terminal.ts";
import {ReactNode, useCallback, useEffect, useRef, useState} from 'react';
import {TerminalHandle} from "@/page/common/dto/terminal/TerminalHandle.ts";
import {RESP_EVENT_SERIAL} from "@/common/model/resp/event/RespEventSerial.ts";
import {useSerialService, useTabManagerService} from "@/common/context/useAppHooks.tsx";
import {createTerminalSettingsAction} from "@/page/protocol/common/terminal/TerminalSettingsAction.tsx";

export interface SerialPageProps {
    portName: string
}

export function SerialPage(props: SerialPageProps): ReactNode {
    const {t, tf} = useI18n();
    const {portName} = props;
    const {paneId} = usePaneContext();
    const openedRef = useRef<boolean>(false);
    const serialService = useSerialService();
    const [, setOpened] = useState<boolean>(false);
    const tabManagerService = useTabManagerService();
    const terminalRef = useRef<TerminalHandle>(null);
    const serialOpenRef = useRef<(() => void) | null>(null);
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
        const payload: SerialOpenEvent = {data: {portName, size: terminal.getWindowSize()}}
        serialOpenRef.current = await serialService.handleConnect(paneId, payload, terminal, setOpened)
    }

    const destroyAll = (): void => {
        destroySerial()
        terminalRef.current?.dispose()
    }

    const destroySerial = (): void => {
        serialOpenRef.current?.();
        serialOpenRef.current = null;
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
        return serialService.addWsListener(paneId, context => {
            if (context.type === RESP_EVENT_SERIAL.ON_ERROR) {
                terminalRef.current?.write(`\r\n\x1b[31m${tf('protocol.serial.error', {error: String(context.payload)})}\x1b[0m\r\n`);
                return;
            }
            if (context.type === RESP_EVENT_SERIAL.ON_CLOSE) {
                terminalRef.current?.write(`\r\n\x1b[33m${t('protocol.serial.closed')}\x1b[0m\r\n`);
            }
        });
    }, [paneId, serialService, t, tf]);

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
            <Terminal ref={terminalRef} settingKey={resolveTerminalSettingKey(portName, TERMINAL_KIND.SERIAL)} closePane={destroyAll}></Terminal>
        </Box>
    );
}
