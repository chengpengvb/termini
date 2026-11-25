import {Ref, useRef, useState} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {Box, ButtonBase, Collapse} from '@mui/material';
import {isNil, isNotNil} from '@/common/util/StringUtil.ts';
import type {AiChatItem} from '@/common/model/AiChatItem.ts';
import {useItemHandle} from '@/page/common/hooks/useItemHandle.ts';
import {AgentCodeBlock} from '@/page/agent/common/AgentCodeBlock.tsx';
import type {ItemHandle} from '@/page/common/dto/agent/ItemHandle.ts';
import {KeyboardArrowDown, KeyboardArrowRight} from '@mui/icons-material';
import {RESP_EVENT_AGENT} from '@/common/model/resp/event/RespEventAgent.ts';
import {ConflictActions} from '@/page/agent/main/body/tool/common/ConflictActions.tsx';
import {AGENT_ANSWER_CODE, isAgentAnswerAllowed} from '@/common/model/req/event/WsAgent.ts';

interface SkillWriteArgsRaw {
    name?: string;
    content?: string;
}

interface ItemSkillWriteProps {
    sessionId: string;
    ref?: Ref<ItemHandle>;
}

export function ItemSkillWrite(props: ItemSkillWriteProps) {
    const {t} = useI18n();
    const {sessionId, ref} = props;
    const [open, setOpen] = useState(false);
    const [done, setDone] = useState(false);
    const conflictPassedRef = useRef(false);
    const [rejected, setRejected] = useState(false);
    const [callId, setCallId] = useState<string | null>(null);
    const [result, setResult] = useState<string | null>(null);
    const [content, setContent] = useState<string | null>(null);
    const [decision, setDecision] = useState<number | null>(null);
    const [skillName, setSkillName] = useState<string | null>(null);

    function markConflictPassed(): void {
        conflictPassedRef.current = true;
    }

    function resolveArgs(args: unknown): void {
        const raw = args as SkillWriteArgsRaw | null;
        if (isNil(raw)) {
            return;
        }
        if (isNotNil(raw.name)) {
            setSkillName(raw.name);
        }
        if (isNotNil(raw.content)) {
            setContent(raw.content);
        }
    }

    function canDecide(): boolean {
        if (conflictPassedRef.current || done || rejected || isNil(callId)) {
            return false;
        }
        return !isNotNil(decision);
    }

    function resolveTitle(): string {
        if (rejected) {
            return t('agent.tool.skillWriteSkipped');
        }
        if (done) {
            return result ?? t('agent.tool.skillWriteDone');
        }
        if (isAgentAnswerAllowed(decision)) {
            return t('agent.tool.skillWriteRunning');
        }
        return t('agent.tool.skillWriteWaiting');
    }

    function resolveTitleClassName(): string {
        if (done || rejected) {
            return 'tm-agent-tool-title';
        }
        return 'tm-agent-tool-title tm-agent-shimmer';
    }

    function handleToggleOpen(): void {
        setOpen((current: boolean) => !current);
    }

    function onEvent(eventItem: AiChatItem): void {
        if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL) {
            setCallId(eventItem.context.callId);
            resolveArgs(eventItem.context.args);
            return;
        }
        if (eventItem.type === RESP_EVENT_AGENT.ON_CONFLICT) {
            if (conflictPassedRef.current || done || isNotNil(decision)) {
                return;
            }
            setCallId(eventItem.context.callId);
            resolveArgs(eventItem.context.args);
            if (eventItem.context.autoAllowed) {
                markConflictPassed();
                setDecision(AGENT_ANSWER_CODE.ALLOW);
                return;
            }
            setOpen(true);
            return;
        }
        if (eventItem.type === RESP_EVENT_AGENT.ON_CONFLICT_ANSWERED) {
            markConflictPassed();
            setCallId(eventItem.context.callId);
            setDecision(eventItem.context.code);
            if (eventItem.context.code === AGENT_ANSWER_CODE.REJECT) {
                setRejected(true);
            }
            return;
        }
        if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL_DONE) {
            markConflictPassed();
            setDone(true);
            setResult(eventItem.context.result);
            return;
        }
        if (eventItem.type === RESP_EVENT_AGENT.ON_DONE || eventItem.type === RESP_EVENT_AGENT.ON_CANCELLED) {
            markConflictPassed();
            if (!rejected) {
                setDone(true);
            }
        }
    }

    useItemHandle(ref, onEvent);

    return (
        <Box className="tm-agent-tool-card">
            <Box className="tm-agent-tool-head">
                <ButtonBase disableRipple className="tm-agent-tool-fold" onClick={handleToggleOpen}>
                    <Box component="span" className="tm-agent-tool-chevron">
                        {open ? <KeyboardArrowDown sx={{fontSize: 16}}/> : <KeyboardArrowRight sx={{fontSize: 16}}/>}
                    </Box>
                    <Box component="span" className={resolveTitleClassName()}>{resolveTitle()}</Box>
                </ButtonBase>
                {skillName ? <Box component="span" className="tm-agent-run-summary-target">{skillName}</Box> : null}
                {canDecide() ? <ConflictActions sessionId={sessionId} callId={callId}/> : null}
            </Box>
            <Collapse in={open} unmountOnExit timeout={0}>
                <Box className="tm-agent-tool-body">
                    {isNotNil(content) ? (
                        <AgentCodeBlock code={content} language="text"/>
                    ) : null}
                </Box>
            </Collapse>
        </Box>
    );
}
