import {save} from '@tauri-apps/plugin-dialog';
import type {AiChat} from '@/common/model/AiChat.ts';
import {isNil, isNotNil} from '@/common/util/StringUtil.ts';
import type {AiChatItem} from '@/common/model/AiChatItem.ts';
import {translate, translateFormat} from '@/common/i18n/i18n.ts';
import type {FileEditService} from '@/service/FileEditService.ts';
import type {AiChatItemService} from '@/service/AiChatItemService.ts';
import {resolveResultMessage} from '@/common/model/resp/ResultModel.ts';
import {RESP_EVENT_AGENT} from '@/common/model/resp/event/RespEventAgent.ts';
import {AgentTurnClock, TURN_END_STATE} from '@/page/common/util/agentTurnClock.ts';
import {AgentRunSummaryBuffer, type RunSummaryLine} from '@/page/common/util/agentRunSummary.ts';
import {formatDuration, formatFullDateTime, formatMessageTime} from '@/page/common/util/agentTimeFormat.ts';

const FILE_NAME_MAX = 60;

function sanitizeFileName(title: string): string {
    const cleaned = title.replace(/[\\/:*?"<>|\r\n]+/g, ' ').replace(/\s+/g, ' ').trim();
    const cut = cleaned.length > FILE_NAME_MAX ? cleaned.slice(0, FILE_NAME_MAX) : cleaned;
    return cut.length > 0 ? cut : 'chat';
}

function isNested(item: AiChatItem): boolean {
    return isNotNil(item.context?.parentReqId);
}

function renderToolLine(line: RunSummaryLine): string {
    const parts = [`\`${line.tool}\``];
    if (line.target) {
        parts.push(line.target);
    }
    if (line.result) {
        parts.push(`→ ${line.result}`);
    }
    if (isNotNil(line.addCount) && isNotNil(line.delCount)) {
        parts.push(`+${line.addCount} -${line.delCount}`);
    }
    if (isNotNil(line.exitCode)) {
        parts.push(`(exit ${line.exitCode})`);
    }
    return `- ${parts.join(' ')}`;
}

function renderError(item: AiChatItem): string {
    const code: number | null = item.context?.code ?? null;
    if (isNotNil(code)) {
        try {
            return resolveResultMessage(code);
        } catch {
            return `code ${code}`;
        }
    }
    return String(item.context?.error ?? '');
}

export function buildChatMarkdown(chat: AiChat, items: AiChatItem[]): string {
    const summary = new AgentRunSummaryBuffer();
    const clock = new AgentTurnClock();
    for (const item of items) {
        clock.accumulate(item);
        if (!isNested(item)) {
            summary.accumulate(item);
        }
    }

    const lines: string[] = [`# ${chat.title}`, '', `> ${translateFormat('agent.export.exportedAt', {time: formatFullDateTime(Date.now())})}`, ''];
    let reqId: string | null = null;
    let toolsWritten = false;

    function flushTools(): void {
        if (toolsWritten || isNil(reqId)) {
            return;
        }
        const toolLines = summary.buildLines(reqId);
        if (toolLines.length > 0) {
            lines.push(`**${translate('agent.export.tools')}**`, '');
            for (const line of toolLines) {
                lines.push(renderToolLine(line));
            }
            lines.push('');
        }
        toolsWritten = true;
    }

    function flushTurnEnd(): void {
        if (isNil(reqId)) {
            return;
        }
        const timing = clock.resolve(reqId);
        if (isNil(timing)) {
            return;
        }
        const stateKey = timing.state === TURN_END_STATE.CANCELLED ? 'agent.turn.cancelled' : timing.state === TURN_END_STATE.FAILED ? 'agent.turn.failed' : 'agent.turn.done';
        lines.push(`_${translate(stateKey)} · ${formatDuration(timing.endAt - timing.startAt)}_`, '');
    }

    for (const item of items) {
        if (isNested(item)) {
            continue;
        }
        if (item.type === RESP_EVENT_AGENT.ON_USER) {
            flushTools();
            flushTurnEnd();
            reqId = item.context?.reqId ?? null;
            toolsWritten = false;
            lines.push(`## ${translate('agent.export.user')} · ${formatMessageTime(item.timestamp)}`, '', String(item.context?.message ?? ''), '');
            continue;
        }
        if (item.type === RESP_EVENT_AGENT.ON_TEXT_FINAL) {
            flushTools();
            const text = String(item.context?.delta ?? '').trim();
            if (text.length > 0) {
                lines.push(`### ${translate('agent.export.assistant')}`, '', text, '');
            }
            continue;
        }
        if (item.type === RESP_EVENT_AGENT.ON_ERROR) {
            flushTools();
            const text = renderError(item).trim();
            if (text.length > 0) {
                lines.push(`> ⚠ ${text}`, '');
            }
        }
    }
    flushTools();
    flushTurnEnd();
    return lines.join('\n');
}

export async function exportChatMarkdown(chat: AiChat, aiChatItemService: AiChatItemService, fileEditService: FileEditService): Promise<string | null> {
    const target = await save({defaultPath: `${sanitizeFileName(chat.title)}.md`, filters: [{name: 'Markdown', extensions: ['md']}]});
    if (isNil(target)) {
        return null;
    }
    const items = await aiChatItemService.selectByChatId(chat.id);
    await fileEditService.write(target, buildChatMarkdown(chat, items), true);
    return target;
}
