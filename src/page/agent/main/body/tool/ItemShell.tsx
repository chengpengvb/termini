import {Ref} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import type {ItemHandle} from '@/page/common/dto/agent/ItemHandle.ts';
import {ShellToolView} from '@/page/agent/main/body/tool/common/ShellToolView.tsx';
import type {ShellConflictView, ShellToolTitles} from '@/page/common/dto/agent/ToolView.ts';

interface ItemShellProps {
    sessionId: string;
    ref?: Ref<ItemHandle>;
}

interface LocalShellCommandRaw {
    command: string;
    cwd?: string | null;
    title?: string | null;
}

interface LocalShellConflictArgsRaw {
    command: string | null;
    cwd: string | null;
    commands?: LocalShellCommandRaw[];
}

function describeCommand(index: number, total: number, raw: LocalShellCommandRaw, topCwd: string | null): string {
    const marks: string[] = [`${index + 1}/${total}`];
    if (raw.title) {
        marks.push(raw.title);
    }
    if (raw.cwd && raw.cwd !== topCwd) {
        marks.push(`cwd: ${raw.cwd}`);
    }
    return `# ${marks.join(' · ')}\n${raw.command}`;
}

export function ItemShell(props: ItemShellProps) {
    const {t} = useI18n();
    const {sessionId, ref} = props;

    function resolveDetail(cwd: string | null): string | null {
        if (!cwd) {
            return null;
        }
        return `cwd: ${cwd}`;
    }

    function resolveConflictArgs(args: unknown): ShellConflictView | null {
        if (!args) {
            return null;
        }
        const raw = args as LocalShellConflictArgsRaw;
        const batch = Array.isArray(raw.commands) ? raw.commands : [];
        if (batch.length > 0) {
            const command = batch.map((item, index) => describeCommand(index, batch.length, item, raw.cwd)).join('\n');
            return {command, detail: resolveDetail(raw.cwd)};
        }
        if (!raw.command) {
            return null;
        }
        return {command: raw.command, detail: resolveDetail(raw.cwd)};
    }

    function resolveTitles(): ShellToolTitles {
        return {
            skipped: t('agent.tool.shellSkipped'),
            completed: t('agent.tool.shellCompleted'),
            running: t('agent.tool.shellRunning'),
            waitingConfirm: t('agent.tool.shellWaiting'),
            preparing: t('agent.tool.shellPreparing'),
        };
    }

    return (
        <ShellToolView sessionId={sessionId} ref={ref} titles={resolveTitles()} codeLanguage="powershell" resolveConflictArgs={resolveConflictArgs}/>
    );
}
