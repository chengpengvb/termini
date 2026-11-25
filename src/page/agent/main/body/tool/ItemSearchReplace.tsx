import {Ref} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import type {ItemHandle} from '@/page/common/dto/agent/ItemHandle.ts';
import {openFileEditTab} from '@/page/common/util/fileEditOpenUtil.ts';
import {useSystemUtilService, useTabManagerService} from '@/common/context/useAppHooks.tsx';
import { SearchReplaceToolView } from '@/page/agent/main/body/tool/common/SearchReplaceToolView.tsx';
import type { ReplaceConflictView, SearchReplaceToolTitles } from '@/page/common/dto/agent/ToolView.ts';

interface ItemSearchReplaceProps {
    sessionId: string;
    ref?: Ref<ItemHandle>;
}

interface LocalReplaceConflictArgs {
    root?: string;
    path?: string;
    old_string?: string;
    new_string?: string;
    replace_all?: boolean;
    start_line?: number;
    edits?: LocalReplaceConflictArgs[];
}

export function ItemSearchReplace(props: ItemSearchReplaceProps) {
    const {t, tf} = useI18n();
    const {sessionId, ref} = props;
    const tabManagerService = useTabManagerService();
    const systemUtilService = useSystemUtilService();

    function resolveFileName(path: string): string {
        const index = Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/'));
        if (index < 0) {
            return path;
        }
        return path.slice(index + 1);
    }

    function isAbsolute(path: string): boolean {
        return path.startsWith('/') || (path.length > 2 && path[1] === ':');
    }

    function resolveFullPath(root: string | undefined, path: string): string {
        if (isAbsolute(path) || !root) {
            return path;
        }
        return `${root.replace(/[/\\]+$/, '')}/${path}`;
    }

    function toView(root: string | undefined, raw: LocalReplaceConflictArgs): ReplaceConflictView | null {
        if (!raw.path) {
            return null;
        }
        return {
            displayName: resolveFileName(raw.path),
            path: raw.path,
            fullPath: resolveFullPath(root, raw.path),
            startLine: raw.start_line ?? null,
            old_string: raw.old_string ?? '',
            new_string: raw.new_string ?? '',
        };
    }

    // 批量编辑一次带回多个文件, 每个文件各自成一条(含自己的 diff), 由视图逐行展示
    function resolveConflictArgs(args: unknown): ReplaceConflictView[] {
        if (!args) {
            return [];
        }
        const raw = args as LocalReplaceConflictArgs;
        if (Array.isArray(raw.edits) && raw.edits.length > 0) {
            return raw.edits.map(edit => toView(raw.root, edit)).filter((view): view is ReplaceConflictView => view !== null);
        }
        const single = toView(raw.root, raw);
        return single === null ? [] : [single];
    }

    function resolveCompletedTitle(displayName: string): string {
        return tf('agent.tool.writeCompletedNamed', {name: displayName});
    }

    function resolveTitles(): SearchReplaceToolTitles {
        return {
            skipped: t('agent.tool.writeSkipped'),
            completedFallback: t('agent.tool.writeCompleted'),
            running: t('agent.tool.writeRunning'),
            waitingConfirm: t('agent.tool.writeWaiting'),
            preparing: t('agent.tool.writePreparing'),
        };
    }

    function handleOpenPath(path: string): void {
        openFileEditTab(tabManagerService, systemUtilService, path);
    }

    return (
        <SearchReplaceToolView sessionId={sessionId} ref={ref} titles={resolveTitles()} resolveCompletedTitle={resolveCompletedTitle} resolveConflictArgs={resolveConflictArgs} onOpenPath={handleOpenPath} />
    );
}
