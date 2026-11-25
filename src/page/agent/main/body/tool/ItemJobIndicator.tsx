import {Ref, useRef, useState} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNil, isNotNil} from '@/common/util/StringUtil.ts';
import type {AiChatItem} from '@/common/model/AiChatItem.ts';
import {Box, Button, ButtonBase, Collapse} from '@mui/material';
import {resolveAiToolLabel} from '@/common/const/AiToolConst.ts';
import {useAgentService} from '@/common/context/useAppHooks.tsx';
import {useItemHandle} from '@/page/common/hooks/useItemHandle.ts';
import {AgentCodeBlock} from '@/page/agent/common/AgentCodeBlock.tsx';
import type {ItemHandle} from '@/page/common/dto/agent/ItemHandle.ts';
import {KeyboardArrowDown, KeyboardArrowRight} from '@mui/icons-material';
import {RESP_EVENT_AGENT} from '@/common/model/resp/event/RespEventAgent.ts';
import {ConflictActions} from '@/page/agent/main/body/tool/common/ConflictActions.tsx';
import {AGENT_ANSWER_CODE, isAgentAnswerAllowed} from '@/common/model/req/event/WsAgent.ts';
import {ConflictAuditNotice} from '@/page/agent/main/body/tool/common/ConflictAuditNotice.tsx';

interface ItemJobIndicatorProps {
    sessionId: string;
    ref?: Ref<ItemHandle>;
}

export function ItemJobIndicator(props: ItemJobIndicatorProps) {
    const {t, tf} = useI18n();
    const {sessionId, ref} = props;
    const agentService = useAgentService();
    const [open, setOpen] = useState(false);
    const [done, setDone] = useState(false);
    const conflictPassedRef = useRef(false);
    const [rejected, setRejected] = useState(false);
    const [reqId, setReqId] = useState<string | null>(null);
    const [runId, setRunId] = useState<string | null>(null);
    const [callId, setCallId] = useState<string | null>(null);
    const [label, setLabel] = useState<string | null>(null);
    const [result, setResult] = useState<string | null>(null);
    const [decision, setDecision] = useState<number | null>(null);
    const [argsText, setArgsText] = useState<string | null>(null);
    const [auditReason, setAuditReason] = useState<string | null>(null);

    function markConflictPassed(): void {
        conflictPassedRef.current = true;
    }

    function resolveLabel(): string {
        if (isNil(label)) {
            return t('agent.tool.jobIndicatorDefault');
        }
        return label;
    }

    function resolveTitle(): string {
        const name = resolveLabel();
        if (rejected) {
            return tf('agent.tool.jobIndicatorSkipped', {name});
        }
        if (done) {
            return tf('agent.tool.jobIndicatorCompleted', {name});
        }
        if (isAgentAnswerAllowed(decision)) {
            return tf('agent.tool.jobIndicatorRunning', {name});
        }
        if (isNotNil(argsText) && !conflictPassedRef.current) {
            return tf('agent.tool.jobIndicatorWaiting', {name});
        }
        return tf('agent.tool.jobIndicatorPreparing', {name});
    }

    function resolveTitleClassName(): string {
        if (done || rejected) {
            return 'tm-agent-tool-title';
        }
        return 'tm-agent-tool-title tm-agent-shimmer';
    }

    function canDecide(): boolean {
        if (conflictPassedRef.current || done || rejected || isNil(argsText)) {
            return false;
        }
        return !isNotNil(decision);
    }

    function canCancel(): boolean {
        if (done || rejected || isNil(decision)) {
            return false;
        }
        return isAgentAnswerAllowed(decision);
    }

    function handleCancel(): void {
        if (isNil(callId) || isNil(reqId) || isNil(runId)) {
            return;
        }
        agentService.handleToolCancel(sessionId, {
            data: {reqId, runId, callId},
        });
    }

    function handleToggleOpen(): void {
        setOpen((current: boolean) => !current);
    }

    function resolveArgsText(args: unknown): string | null {
        if (!args) {
            return null;
        }
        return JSON.stringify(args, null, 2);
    }

    function onEvent(eventItem: AiChatItem): void {
        if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL) {
            setReqId(eventItem.context.reqId);
            setRunId(eventItem.context.runId);
            setCallId(eventItem.context.callId);
            setLabel(resolveAiToolLabel(eventItem.context.toolId));
            return;
        }
        if (eventItem.type === RESP_EVENT_AGENT.ON_CONFLICT) {
            if (conflictPassedRef.current || done || isNotNil(decision)) {
                return;
            }
            setReqId(eventItem.context.reqId);
            setRunId(eventItem.context.runId);
            setCallId(eventItem.context.callId);
            setLabel(resolveAiToolLabel(eventItem.context.toolId));
            setAuditReason(eventItem.context.auditReason ?? null);
            setArgsText(resolveArgsText(eventItem.context.args));
            if (eventItem.context.autoAllowed) {
                markConflictPassed();
                setDecision(AGENT_ANSWER_CODE.ALLOW);
                setOpen(true);
            }
            return;
        }
        if (eventItem.type === RESP_EVENT_AGENT.ON_CONFLICT_ANSWERED) {
            markConflictPassed();
            setCallId(eventItem.context.callId);
            setDecision(eventItem.context.code);
            if (eventItem.context.code === AGENT_ANSWER_CODE.REJECT) {
                setRejected(true);
                return;
            }
            setOpen(true);
            return;
        }
        if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL_DONE) {
            markConflictPassed();
            setDone(true);
            setOpen(false);
            setResult(eventItem.context.result);
            return;
        }
        if (eventItem.type === RESP_EVENT_AGENT.ON_DONE || eventItem.type === RESP_EVENT_AGENT.ON_CANCELLED) {
            markConflictPassed();
            if (rejected) {
                return;
            }
            setDone(true);
            setOpen(false);
        }
    }

    useItemHandle(ref, onEvent);

    if (canDecide()) {
        return (
            <Box className="tm-agent-tool-card">
                <Box className="tm-agent-tool-head">
                    <Box className={resolveTitleClassName()}>{resolveTitle()}</Box>
                    <ConflictActions sessionId={sessionId} callId={callId}/>
                </Box>
                <ConflictAuditNotice reason={auditReason}/>
                {isNotNil(argsText) ? (
                    <Box className="tm-agent-tool-command">
                        <Box sx={{maxHeight: 240, overflow: 'auto'}}>
                            <AgentCodeBlock code={argsText} language="json"/>
                        </Box>
                    </Box>
                ) : null}
            </Box>
        );
    }

    return (
        <Box className="tm-agent-tool-card">
            <Box className="tm-agent-tool-head">
                <ButtonBase disableRipple className="tm-agent-tool-fold" onClick={handleToggleOpen}>
                    <Box component="span" className="tm-agent-tool-chevron">
                        {open ? <KeyboardArrowDown sx={{fontSize: 16}}/> : <KeyboardArrowRight sx={{fontSize: 16}}/>}
                    </Box>
                    <Box component="span" className={resolveTitleClassName()}>{resolveTitle()}</Box>
                </ButtonBase>
                {canCancel() ? (
                    <Button disableRipple className="tm-btn tm-btn-ghost tm-btn-compact" onClick={handleCancel}>
                        {t('agent.tool.cancel')}
                    </Button>
                ) : null}
            </Box>
            <Collapse in={open} unmountOnExit timeout={0}>
                <Box className="tm-agent-tool-body">
                    {isNotNil(argsText) ? (
                        <Box className="tm-agent-tool-command">
                            <Box sx={{maxHeight: 240, overflow: 'auto'}}>
                                <AgentCodeBlock code={argsText} language="json"/>
                            </Box>
                        </Box>
                    ) : null}
                    {isNotNil(result) ? (
                        <Box component="pre" className="tm-agent-shell-output-static">{result}</Box>
                    ) : null}
                </Box>
            </Collapse>
        </Box>
    );
}
