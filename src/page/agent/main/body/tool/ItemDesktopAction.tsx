import {Ref} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import type {ItemHandle} from '@/page/common/dto/agent/ItemHandle.ts';
import {ShellToolView} from '@/page/agent/main/body/tool/common/ShellToolView.tsx';
import type {ShellConflictView, ShellToolTitles} from '@/page/common/dto/agent/ToolView.ts';

interface ItemDesktopActionProps {
    sessionId: string;
    ref?: Ref<ItemHandle>;
}

interface DesktopActionArgsRaw {
    action?: string;
    x?: number;
    y?: number;
    button?: string;
    double?: boolean;
    text?: string;
    keys?: string;
    amount?: number;
    horizontal?: boolean;
    window_id?: string;
    wait_ms?: number;
    actions?: DesktopActionArgsRaw[];
}

function describeAction(raw: DesktopActionArgsRaw): string {
    const action = (raw.action ?? '').trim().toLowerCase();
    let body: string;
    switch (action) {
        case 'click': {
            const button = raw.button ? `${raw.button} ` : '';
            const suffix = raw.double ? ' x2' : '';
            body = `click ${button}(${raw.x ?? '?'}, ${raw.y ?? '?'})${suffix}`;
            break;
        }
        case 'move':
            body = `move (${raw.x ?? '?'}, ${raw.y ?? '?'})`;
            break;
        case 'scroll':
            body = `scroll ${raw.horizontal ? 'x' : 'y'} ${raw.amount ?? 0}`;
            break;
        case 'type':
            body = `type ${JSON.stringify(raw.text ?? '')}`;
            break;
        case 'key':
            body = `key ${raw.keys ?? ''}`;
            break;
        case 'focus':
            body = `focus ${raw.window_id ?? ''}`;
            break;
        case 'wait':
            body = 'wait';
            break;
        default:
            body = action || 'desktop';
            break;
    }
    if (raw.wait_ms !== undefined) {
        return `${body} wait ${raw.wait_ms}ms`;
    }
    return body;
}

export function ItemDesktopAction(props: ItemDesktopActionProps) {
    const {t} = useI18n();
    const {sessionId, ref} = props;

    function resolveConflictArgs(args: unknown): ShellConflictView | null {
        if (!args) {
            return null;
        }
        const raw = args as DesktopActionArgsRaw;
        const command = Array.isArray(raw.actions) && raw.actions.length > 0 ? raw.actions.map(describeAction).join('\n') : describeAction(raw);
        return {command, detail: null};
    }

    function resolveTitles(): ShellToolTitles {
        return {
            skipped: t('agent.tool.desktopSkipped'),
            completed: t('agent.tool.desktopCompleted'),
            running: t('agent.tool.desktopRunning'),
            waitingConfirm: t('agent.tool.desktopWaiting'),
            preparing: t('agent.tool.desktopPreparing'),
        };
    }

    return (
        <ShellToolView sessionId={sessionId} ref={ref} titles={resolveTitles()} codeLanguage="text" resolveConflictArgs={resolveConflictArgs}/>
    );
}
