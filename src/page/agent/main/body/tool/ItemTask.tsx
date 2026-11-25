import {useI18n} from '@/common/i18n/useI18n.ts';
import {Box, Button, ButtonBase} from '@mui/material';
import {isNil, isNotNil} from '@/common/util/StringUtil.ts';
import {AI_TOOL_STATUS} from '@/common/const/AiToolConst.ts';
import type {AiChatItem} from '@/common/model/AiChatItem.ts';
import {useAgentService} from '@/common/context/useAppHooks.tsx';
import {AgentEventHub} from '@/page/common/util/AgentEventHub.ts';
import {useItemHandle} from '@/page/common/hooks/useItemHandle.ts';
import {Ref, useCallback, useEffect, useRef, useState} from 'react';
import type {LegacyStep} from '@/page/common/dto/agent/ToolView.ts';
import type {ItemHandle} from '@/page/common/dto/agent/ItemHandle.ts';
import {buildAgentRoot} from '@/page/common/util/agentMessageHistory.ts';
import {RESP_EVENT_AGENT} from '@/common/model/resp/event/RespEventAgent.ts';
import {ItemTaskBody} from '@/page/agent/main/body/tool/task/ItemTaskBody.tsx';
import {dropTaskPhase, publishTaskPhase} from '@/page/common/util/taskPhaseStore.ts';
import {ConflictActions} from '@/page/agent/main/body/tool/common/ConflictActions.tsx';
import {resolveToolStepTitle, summarizeToolArgs} from '@/page/common/util/toolViewUtil.ts';

interface ItemTaskProps {
    itemId: string;
    sessionId: string;
    ref?: Ref<ItemHandle>;
}

const PIXEL_SPINNER_DOTS = [0, 1, 2, 3, 4, 5];

const TASK_PHASE = {
    RUNNING: 0,
    DETACHED: 1,
    DONE: 2,
    KILLED: 3,
    FAILED: 4,
    CANCELLED: 5,
} as const;

type TaskPhase = (typeof TASK_PHASE)[keyof typeof TASK_PHASE];

function resolvePhase(status: number | undefined): TaskPhase {
    if (status === AI_TOOL_STATUS.DETACHED || status === AI_TOOL_STATUS.DONE || status === AI_TOOL_STATUS.KILLED || status === AI_TOOL_STATUS.FAILED || status === AI_TOOL_STATUS.CANCELLED) {
        return status;
    }
    return TASK_PHASE.DONE;
}

function resolveNote(result: string, phase: TaskPhase): string | null {
    if (phase !== TASK_PHASE.KILLED && phase !== TASK_PHASE.FAILED) {
        return null;
    }
    const first = result.split('\n')[0];
    const cut = first.indexOf(':');
    return cut >= 0 ? first.slice(cut + 1).trim() : first;
}

export function ItemTask(props: ItemTaskProps) {
    const {t} = useI18n();
    const {itemId, sessionId, ref} = props;
    const agentService = useAgentService();
    const [open, setOpen] = useState(false);
    const subReqIdRef = useRef<string | null>(null);
    const stepCallIdRef = useRef<string | null>(null);
    const nestedHubRef = useRef(new AgentEventHub());
    const [note, setNote] = useState<string | null>(null);
    const phaseRef = useRef<TaskPhase>(TASK_PHASE.RUNNING);
    const [title, setTitle] = useState<string | null>(null);
    const [reqId, setReqId] = useState<string | null>(null);
    const [runId, setRunId] = useState<string | null>(null);
    const [callId, setCallId] = useState<string | null>(null);
    const [prompt, setPrompt] = useState<string | null>(null);
    const [startedAt, setStartedAt] = useState<number | null>(null);
    const [stepTitle, setStepTitle] = useState<string | null>(null);
    const [legacySteps, setLegacySteps] = useState<LegacyStep[]>([]);
    const [nestedRoots, setNestedRoots] = useState<AiChatItem[]>([]);
    const [pendingApproval, setPendingApproval] = useState<AiChatItem | null>(null);
    const [phase, setPhaseState] = useState<TaskPhase>(TASK_PHASE.RUNNING);

    function setPhase(next: TaskPhase): void {
        phaseRef.current = next;
        setPhaseState(next);
    }

    const registerNested = useCallback((id: string, handle: ItemHandle | null): void => {
        nestedHubRef.current.register(id, handle);
    }, []);

    const running = phase === TASK_PHASE.RUNNING || phase === TASK_PHASE.DETACHED;

    useEffect(() => {
        publishTaskPhase(itemId, {running, startedAt});
    }, [itemId, running, startedAt]);

    useEffect(() => {
        return () => {
            dropTaskPhase(itemId);
        };
    }, [itemId]);

    function resolvePromptTitle(text: string): string {
        const first = text.split('\n')[0].trim();
        const cut = first.split(/[：:]/)[0].trim();
        return cut.length > 0 ? cut : first;
    }

    function resolveTitle(): string {
        if (isNotNil(title)) {
            return title;
        }
        if (isNotNil(prompt)) {
            return resolvePromptTitle(prompt);
        }
        return resolveStatus();
    }

    function resolveStatus(): string {
        if (isNotNil(pendingApproval)) {
            const summary = summarizeToolArgs(pendingApproval.context.args);
            return summary.length > 0 ? summary : pendingApproval.context.toolId;
        }
        if (phase === TASK_PHASE.RUNNING && isNotNil(stepTitle)) {
            return stepTitle;
        }
        switch (phase) {
            case TASK_PHASE.CANCELLED:
                return t('agent.tool.taskCancelled');
            case TASK_PHASE.DETACHED:
                return t('agent.tool.taskDetached');
            case TASK_PHASE.KILLED:
                return t('agent.tool.taskKilled');
            case TASK_PHASE.FAILED:
                return t('agent.tool.taskFailed');
            case TASK_PHASE.DONE:
                return t('agent.tool.taskDone');
            default:
                return nestedRoots.length > 0 || legacySteps.length > 0 ? t('agent.tool.taskRunning') : t('agent.tool.taskPlanning');
        }
    }

    function isRunning(): boolean {
        return running;
    }

    function canCancel(): boolean {
        return isRunning() && isNotNil(callId) && isNotNil(reqId) && isNotNil(runId);
    }

    function handleCancel(): void {
        if (isNil(callId) || isNil(reqId) || isNil(runId)) {
            return;
        }
        agentService.handleToolCancel(sessionId, {data: {reqId, runId, callId}});
        setPhase(TASK_PHASE.CANCELLED);
    }

    function handleToggleOpen(): void {
        setOpen((current: boolean) => !current);
    }

    function acceptNested(item: AiChatItem): void {
        const hub = nestedHubRef.current;
        const target = hub.resolve(item);
        if (isNil(target)) {
            return;
        }
        if (!hub.hasRoot(target.rootId)) {
            hub.addRoot(target.rootId);
            setNestedRoots(prev => [...prev, buildAgentRoot(item, target)]);
        } else if (target.self) {
            setNestedRoots(prev => prev.map(row => (row.id === item.id && row.type !== item.type ? {...row, type: item.type, context: item.context, timestamp: item.timestamp} : row)));
        }
        hub.dispatch(target.rootId, item);
    }

    function isRunEnd(eventItem: AiChatItem): boolean {
        return eventItem.type === RESP_EVENT_AGENT.ON_DONE || eventItem.type === RESP_EVENT_AGENT.ON_CANCELLED;
    }

    function isNestedEvent(eventItem: AiChatItem): boolean {
        if (isNotNil(eventItem.context?.parentReqId)) {
            return true;
        }
        const subReqId = subReqIdRef.current;
        return isNotNil(subReqId) && eventItem.context?.reqId === subReqId;
    }

    function endNested(eventItem: AiChatItem): void {
        const subReqId = subReqIdRef.current;
        if (isNil(subReqId)) {
            return;
        }
        const hub = nestedHubRef.current;
        hub.addRunEnd(subReqId, eventItem);
        hub.broadcast(eventItem);
    }

    function applyNestedApproval(eventItem: AiChatItem): void {
        if (eventItem.type === RESP_EVENT_AGENT.ON_CONFLICT) {
            if (eventItem.context.autoAllowed === true) {
                return;
            }
            setPendingApproval(eventItem);
            return;
        }
        if (eventItem.type === RESP_EVENT_AGENT.ON_CONFLICT_ANSWERED) {
            setPendingApproval((prev: AiChatItem | null) => (isNotNil(prev) && prev.context.callId === eventItem.context.callId ? null : prev));
        }
    }

    function applyNestedStep(eventItem: AiChatItem): void {
        if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL) {
            stepCallIdRef.current = eventItem.context.callId;
            setStepTitle(resolveToolStepTitle(eventItem.context.toolId, eventItem.context.args));
            return;
        }
        if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL_DONE && stepCallIdRef.current === eventItem.context.callId) {
            stepCallIdRef.current = null;
            setStepTitle(null);
        }
    }

    function clearNestedStep(): void {
        stepCallIdRef.current = null;
        setStepTitle(null);
    }

    function onEvent(eventItem: AiChatItem): void {
        if (isNestedEvent(eventItem)) {
            if (isRunEnd(eventItem)) {
                setPendingApproval(null);
                clearNestedStep();
                endNested(eventItem);
                return;
            }
            applyNestedApproval(eventItem);
            applyNestedStep(eventItem);
            acceptNested(eventItem);
            return;
        }
        if (eventItem.type === RESP_EVENT_AGENT.ON_SUBAGENT_OPEN) {
            subReqIdRef.current = eventItem.context.subReqId ?? null;
            if (eventItem.context.title) {
                setTitle(eventItem.context.title);
            }
            return;
        }
        if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL) {
            setReqId(eventItem.context.reqId);
            setRunId(eventItem.context.runId);
            setCallId(eventItem.context.callId);
            setStartedAt(eventItem.timestamp);
            if (eventItem.context.args?.prompt) {
                setPrompt(eventItem.context.args.prompt);
            }
            if (eventItem.context.args?.title) {
                setTitle(eventItem.context.args.title);
            }
            return;
        }
        if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL_DONE) {
            const result: string = eventItem.context.result ?? '';
            const next = resolvePhase(eventItem.context.status);
            setPhase(next);
            setNote(resolveNote(result, next));
            return;
        }
        if (eventItem.type === RESP_EVENT_AGENT.ON_SUBAGENT_TOOL_CALL) {
            const step: LegacyStep = {subCallId: eventItem.context.subCallId, toolId: eventItem.context.toolId, args: summarizeToolArgs(eventItem.context.args)};
            setLegacySteps(prev => (prev.some(item => item.subCallId === step.subCallId) ? prev : [...prev, step]));
            return;
        }
        if (eventItem.type === RESP_EVENT_AGENT.ON_SUBAGENT_TOOL_DONE) {
            const subCallId = eventItem.context.subCallId;
            setLegacySteps(prev => prev.map(item => (item.subCallId === subCallId ? {...item, ok: eventItem.context.ok === true, elapsedMs: eventItem.context.elapsedMs} : item)));
            return;
        }
        if (eventItem.type === RESP_EVENT_AGENT.ON_CANCELLED) {
            setPhase(TASK_PHASE.CANCELLED);
            endNested(eventItem);
            return;
        }
        if (eventItem.type === RESP_EVENT_AGENT.ON_DONE) {
            if (phaseRef.current !== TASK_PHASE.DETACHED) {
                endNested(eventItem);
            }
            if (phaseRef.current === TASK_PHASE.RUNNING) {
                setPhase(TASK_PHASE.DONE);
            }
        }
    }

    useItemHandle(ref, onEvent);

    return (
        <Box className="tm-agent-tool-card tm-agent-tool-task">
            <Box className="tm-agent-tool-head">
                <ButtonBase disableRipple className="tm-agent-tool-fold" onClick={handleToggleOpen}>
                    <Box component="span" className="tm-agent-tool-chevron">
                        {isRunning() ? (
                            <Box component="span" className="tm-agent-tool-task-spinner" aria-hidden="true">
                                {PIXEL_SPINNER_DOTS.map(index => (
                                    <Box component="span" key={index} className="tm-agent-tool-task-spinner-dot"/>
                                ))}
                            </Box>
                        ) : (
                            <Box component="span" className="tm-agent-tool-task-dot"/>
                        )}
                    </Box>
                    <Box className="tm-agent-tool-task-copy">
                        <Box className="tm-agent-tool-task-line">
                            <Box component="span" className="tm-agent-tool-title" title={isNotNil(prompt) ? prompt : undefined}>
                                {resolveTitle()}
                            </Box>
                            <Box component="span" className="tm-agent-tool-role">
                                {t('agent.tool.taskExplorer')}
                            </Box>
                        </Box>
                        <Box component="span" className={phase === TASK_PHASE.KILLED || phase === TASK_PHASE.FAILED ? 'tm-agent-tool-status is-failed' : 'tm-agent-tool-status'}>
                            {resolveStatus()}
                        </Box>
                    </Box>
                </ButtonBase>
                {isNotNil(pendingApproval) ? (
                    <ConflictActions sessionId={sessionId} callId={pendingApproval.context.callId} sessionPrefix={pendingApproval.context.sessionPrefix ?? null}/>
                ) : canCancel() ? (
                    <Button disableRipple className="tm-btn tm-btn-secondary tm-btn-compact tm-agent-tool-task-stop" onClick={handleCancel}>
                        {t('agent.tool.cancel')}
                    </Button>
                ) : null}
            </Box>
            <ItemTaskBody open={open} note={note} sessionId={sessionId} legacySteps={legacySteps} nestedRoots={nestedRoots} onRegisterNested={registerNested}/>
        </Box>
    );
}
