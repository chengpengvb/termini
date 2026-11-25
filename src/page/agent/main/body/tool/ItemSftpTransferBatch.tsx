import {Ref} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNil} from '@/common/util/StringUtil.ts';
import {useDeviceService} from '@/common/context/useAppHooks.tsx';
import {SFTP_TRANSFER_DIRECTION} from '@/common/const/AiToolConst.ts';
import type {ItemHandle} from '@/page/common/dto/agent/ItemHandle.ts';
import {resolveDeviceListDetail} from '@/page/common/util/deviceDetail.ts';
import {ShellToolView} from '@/page/agent/main/body/tool/common/ShellToolView.tsx';
import type {ShellConflictView, ShellToolTitles} from '@/page/common/dto/agent/ToolView.ts';

interface ItemSftpTransferBatchProps {
    sessionId: string;
    ref?: Ref<ItemHandle>;
}

interface SftpTransferBatchConflictArgsRaw {
    device_ids: string[];
    local_path: string;
    remote_path: string;
    force_overwrite: boolean;
    direction: number;
}

export function ItemSftpTransferBatch(props: ItemSftpTransferBatchProps) {
    const {t} = useI18n();
    const {sessionId, ref} = props;
    const deviceService = useDeviceService();

    function resolveConflictArgs(args: unknown): ShellConflictView | null {
        if (!args) {
            return null;
        }
        const raw = args as SftpTransferBatchConflictArgsRaw;
        if (!raw.local_path || !raw.remote_path) {
            return null;
        }
        const download = raw.direction === SFTP_TRANSFER_DIRECTION.DOWNLOAD;
        if (!download && raw.direction !== SFTP_TRANSFER_DIRECTION.UPLOAD) {
            return null;
        }
        const devices = resolveDeviceListDetail(deviceService, raw.device_ids);
        const overwrite = raw.force_overwrite ? 'force_overwrite=true' : 'force_overwrite=false';
        return {
            command: download ? `${raw.remote_path} -> ${raw.local_path}` : `${raw.local_path} -> ${raw.remote_path}`,
            detail: isNil(devices) ? null : `${download ? 'download' : 'upload'} ${devices} ${overwrite}`,
        };
    }

    function resolveTitles(): ShellToolTitles {
        return {
            skipped: t('agent.tool.sftpTransferSkipped'),
            completed: t('agent.tool.sftpTransferCompleted'),
            running: t('agent.tool.sftpTransferRunning'),
            waitingConfirm: t('agent.tool.sftpTransferWaiting'),
            preparing: t('agent.tool.sftpTransferPreparing'),
        };
    }

    return (
        <ShellToolView sessionId={sessionId} ref={ref} titles={resolveTitles()} codeLanguage="text" deviceBatch resolveConflictArgs={resolveConflictArgs}/>
    );
}
