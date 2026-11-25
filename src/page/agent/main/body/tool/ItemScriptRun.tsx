import {Ref, useState} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import type {ItemHandle} from '@/page/common/dto/agent/ItemHandle.ts';
import {ShellToolView} from '@/page/agent/main/body/tool/common/ShellToolView.tsx';
import type {ShellConflictView, ShellToolTitles} from '@/page/common/dto/agent/ToolView.ts';

interface ItemScriptRunProps {
    sessionId: string;
    ref?: Ref<ItemHandle>;
}

interface ScriptRunConflictArgsRaw {
    code: string;
    runtime: string | null;
    deps: string[] | null;
    cwd: string | null;
    args: string[] | null;
}

const RUNTIME_LANGUAGE: Record<string, string> = {
    deno: 'typescript',
    node: 'javascript',
    python: 'python',
};

export function ItemScriptRun(props: ItemScriptRunProps) {
    const {t} = useI18n();
    const {sessionId, ref} = props;
    const [language, setLanguage] = useState<string>('typescript');

    function resolveConflictArgs(args: unknown): ShellConflictView | null {
        if (!args) {
            return null;
        }
        const raw = args as ScriptRunConflictArgsRaw;
        if (!raw.code) {
            return null;
        }
        const details: string[] = [];
        if (raw.runtime) {
            details.push(`runtime: ${raw.runtime}`);
            setLanguage(RUNTIME_LANGUAGE[raw.runtime] ?? 'typescript');
        }
        if (raw.deps && raw.deps.length > 0) {
            details.push(`deps: ${raw.deps.join(' ')}`);
        }
        if (raw.cwd) {
            details.push(`cwd: ${raw.cwd}`);
        }
        if (raw.args && raw.args.length > 0) {
            details.push(`args: ${raw.args.join(' ')}`);
        }
        return {command: raw.code, detail: details.length > 0 ? details.join('\n') : null};
    }

    function resolveTitles(): ShellToolTitles {
        return {
            skipped: t('agent.tool.scriptSkipped'),
            completed: t('agent.tool.scriptCompleted'),
            running: t('agent.tool.scriptRunning'),
            waitingConfirm: t('agent.tool.scriptWaiting'),
            preparing: t('agent.tool.scriptPreparing'),
        };
    }

    return (
        <ShellToolView sessionId={sessionId} ref={ref} titles={resolveTitles()} codeLanguage={language} resolveConflictArgs={resolveConflictArgs}/>
    );
}
