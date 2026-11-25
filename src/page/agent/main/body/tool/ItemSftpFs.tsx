import {Ref} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNil} from '@/common/util/StringUtil.ts';
import {useDeviceService} from '@/common/context/useAppHooks.tsx';
import type {ItemHandle} from '@/page/common/dto/agent/ItemHandle.ts';
import {resolveDeviceListDetail} from '@/page/common/util/deviceDetail.ts';
import {ShellToolView} from '@/page/agent/main/body/tool/common/ShellToolView.tsx';
import type {ShellConflictView, ShellToolTitles} from '@/page/common/dto/agent/ToolView.ts';

interface ItemSftpFsProps {
    sessionId: string;
    ref?: Ref<ItemHandle>;
}

interface SftpFsConflictArgsRaw {
    op: string;
    device_id: string;
    path?: string;
    name?: string;
    names?: string[];
    from?: string;
    to?: string;
    kind?: string;
    mode?: number;
}

export function ItemSftpFs(props: ItemSftpFsProps) {
    const {t} = useI18n();
    const {sessionId, ref} = props;
    const deviceService = useDeviceService();

    function resolveConflictCommand(raw: SftpFsConflictArgsRaw): string | null {
        if (!raw.op) {
            return null;
        }
        if (raw.op === 'rename') {
            if (!raw.from || !raw.to) {
                return null;
            }
            return `rename ${raw.from} -> ${raw.to}`;
        }
        if (raw.op === 'delete') {
            if (!raw.path) {
                return null;
            }
            if (raw.names && raw.names.length > 0) {
                return `delete ${raw.path} ${raw.names.join(' ')}`;
            }
            return `delete ${raw.path}`;
        }
        if (raw.op === 'create') {
            if (!raw.path || !raw.kind) {
                return null;
            }
            if (raw.name) {
                return `create ${raw.kind} ${raw.path}/${raw.name}`;
            }
            return `create ${raw.kind} ${raw.path}`;
        }
        if (raw.op === 'chmod') {
            if (!raw.path || raw.mode === undefined) {
                return null;
            }
            if (raw.names && raw.names.length > 0) {
                return `chmod ${raw.mode} ${raw.path} ${raw.names.join(' ')}`;
            }
            return `chmod ${raw.mode} ${raw.path}`;
        }
        if (raw.op === 'list') {
            if (!raw.path) {
                return null;
            }
            return `list ${raw.path}`;
        }
        return null;
    }

    function resolveConflictArgs(args: unknown): ShellConflictView | null {
        if (!args) {
            return null;
        }
        const raw = args as SftpFsConflictArgsRaw;
        if (!raw.device_id) {
            return null;
        }
        const command = resolveConflictCommand(raw);
        if (!command) {
            return null;
        }
        const devices = resolveDeviceListDetail(deviceService, [raw.device_id]);
        return {
            command,
            detail: isNil(devices) ? null : devices,
        };
    }

    function resolveTitles(): ShellToolTitles {
        return {
            skipped: t('agent.tool.sftpFsSkipped'),
            completed: t('agent.tool.sftpFsCompleted'),
            running: t('agent.tool.sftpFsRunning'),
            waitingConfirm: t('agent.tool.sftpFsWaiting'),
            preparing: t('agent.tool.sftpFsPreparing'),
        };
    }

    return (
        <ShellToolView sessionId={sessionId} ref={ref} titles={resolveTitles()} codeLanguage="text" resolveConflictArgs={resolveConflictArgs}/>
    );
}
