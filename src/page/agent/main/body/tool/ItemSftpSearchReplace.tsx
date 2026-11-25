import {Ref} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {useDeviceService} from '@/common/context/useAppHooks.tsx';
import type {ItemHandle} from '@/page/common/dto/agent/ItemHandle.ts';
import {SearchReplaceToolView} from '@/page/agent/main/body/tool/common/SearchReplaceToolView.tsx';
import type {ReplaceConflictView, SearchReplaceToolTitles} from '@/page/common/dto/agent/ToolView.ts';

interface ItemSftpSearchReplaceProps {
    sessionId: string;
    ref?: Ref<ItemHandle>;
}

interface SftpReplaceConflictArgs {
    path?: string;
    device_id?: string;
    old_string?: string;
    new_string?: string;
    replace_all?: boolean;
    start_line?: number;
    edits?: SftpReplaceConflictArgs[];
}

export function ItemSftpSearchReplace(props: ItemSftpSearchReplaceProps) {
    const {t, tf} = useI18n();
    const {sessionId, ref} = props;
    const deviceService = useDeviceService();

    function resolveFileName(path: string): string {
        const index = path.lastIndexOf('/');
        if (index < 0) {
            return path;
        }
        return path.slice(index + 1);
    }

    function resolveDisplayName(deviceId: string, path: string): string {
        const fileName = resolveFileName(path);
        const deviceLabel = deviceService.resolveDeviceLabel(deviceId);
        if (!deviceLabel) {
            return fileName;
        }
        return `${deviceLabel}:${fileName}`;
    }

    function toView(topDevice: string | undefined, raw: SftpReplaceConflictArgs): ReplaceConflictView | null {
        if (!raw.path) {
            return null;
        }
        const deviceId = raw.device_id ?? topDevice;
        if (!deviceId) {
            return null;
        }
        return {
            path: raw.path,
            fullPath: `${deviceId}:${raw.path}`,
            startLine: raw.start_line ?? null,
            old_string: raw.old_string ?? '',
            new_string: raw.new_string ?? '',
            displayName: resolveDisplayName(deviceId, raw.path),
        };
    }

    function resolveConflictArgs(args: unknown): ReplaceConflictView[] {
        if (!args) {
            return [];
        }
        const raw = args as SftpReplaceConflictArgs;
        if (Array.isArray(raw.edits) && raw.edits.length > 0) {
            return raw.edits.map(edit => toView(raw.device_id, edit)).filter((view): view is ReplaceConflictView => view !== null);
        }
        const single = toView(raw.device_id, raw);
        return single === null ? [] : [single];
    }

    function resolveCompletedTitle(displayName: string): string {
        return tf('agent.tool.sftpWriteCompletedNamed', {name: displayName});
    }

    function resolveTitles(): SearchReplaceToolTitles {
        return {
            skipped: t('agent.tool.sftpWriteSkipped'),
            completedFallback: t('agent.tool.sftpWriteCompleted'),
            running: t('agent.tool.sftpWriteRunning'),
            waitingConfirm: t('agent.tool.sftpWriteWaiting'),
            preparing: t('agent.tool.sftpWritePreparing'),
        };
    }

    return (
        <SearchReplaceToolView sessionId={sessionId} ref={ref} titles={resolveTitles()} resolveCompletedTitle={resolveCompletedTitle} resolveConflictArgs={resolveConflictArgs} />
    );
}
