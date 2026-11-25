import {Ref} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {useDeviceService} from '@/common/context/useAppHooks.tsx';
import type {ItemHandle} from '@/page/common/dto/agent/ItemHandle.ts';
import {resolveDeviceListDetail} from '@/page/common/util/deviceDetail.ts';
import {ShellToolView} from '@/page/agent/main/body/tool/common/ShellToolView.tsx';
import type {ShellConflictView, ShellToolTitles} from '@/page/common/dto/agent/ToolView.ts';

interface ItemTelnetShellBatchProps {
    sessionId: string;
    ref?: Ref<ItemHandle>;
}

interface TelnetShellBatchConflictArgsRaw {
    command: string;
    device_ids: string[];
}

export function ItemTelnetShellBatch(props: ItemTelnetShellBatchProps) {
    const {t} = useI18n();
    const {sessionId, ref} = props;
    const deviceService = useDeviceService();

    function resolveConflictArgs(args: unknown): ShellConflictView | null {
        if (!args) {
            return null;
        }
        const raw = args as TelnetShellBatchConflictArgsRaw;
        if (!raw.command) {
            return null;
        }
        return {
            command: raw.command,
            detail: resolveDeviceListDetail(deviceService, raw.device_ids),
        };
    }

    function resolveTitles(): ShellToolTitles {
        return {
            skipped: t('agent.tool.telnetSkipped'),
            completed: t('agent.tool.telnetCompleted'),
            running: t('agent.tool.telnetRunning'),
            waitingConfirm: t('agent.tool.telnetWaiting'),
            preparing: t('agent.tool.telnetPreparing'),
        };
    }

    return (
        <ShellToolView sessionId={sessionId} ref={ref} titles={resolveTitles()} codeLanguage="text" deviceBatch resolveConflictArgs={resolveConflictArgs}/>
    );
}
