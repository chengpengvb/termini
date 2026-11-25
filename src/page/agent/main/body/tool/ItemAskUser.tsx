import { Check } from '@mui/icons-material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { isNil } from '@/common/util/StringUtil.ts';
import { usePaneContext } from '@/common/components/tab';
import type { AiChatItem } from '@/common/model/AiChatItem.ts';
import { Box, Button, ButtonBase, InputBase } from '@mui/material';
import { useItemHandle } from '@/page/common/hooks/useItemHandle.ts';
import { useTabManagerService } from '@/common/context/useAppHooks.tsx';
import type { ItemHandle } from '@/page/common/dto/agent/ItemHandle.ts';
import { Ref, useEffect, useRef, useState, type ReactNode } from 'react';
import type { RespAgentQuestionItem } from '@/common/model/resp/RespAgent.ts';
import { RESP_EVENT_AGENT } from '@/common/model/resp/event/RespEventAgent.ts';
import type { AgentQuestionAnswer, AgentQuestionAnswerItem } from '@/common/model/req/event/WsAgent.ts';

interface ItemAskUserProps {
  sessionId: string;
  ref?: Ref<ItemHandle>;
  onAskAnswer?: (answer: AgentQuestionAnswer) => void;
}

const ASK_STATE = {
  ANSWERED: 'answered',
  SKIPPED: 'skipped',
  DECLINED: 'declined',
} as const;

function optionLetter(index: number): string {
  return String.fromCharCode(65 + index);
}

export function ItemAskUser(props: ItemAskUserProps) {
  const { t } = useI18n();
  const { paneId } = usePaneContext();
  const tabManagerService = useTabManagerService();
  const { ref, onAskAnswer } = props;
  const callIdRef = useRef<string | null>(null);
  const [expired, setExpired] = useState(false);
  const [skipped, setSkipped] = useState(false);
  const [answered, setAnswered] = useState(false);
  const [otherText, setOtherText] = useState<Record<string, string>>({});
  const [questions, setQuestions] = useState<RespAgentQuestionItem[]>([]);
  const [selectedIds, setSelectedIds] = useState<Record<string, string[]>>({});

  const interactive = !answered && !skipped && !expired;

  function resolveTitle(): string {
    if (expired) {
      return t('agent.tool.askDeclined');
    }
    if (skipped) {
      return t('agent.tool.askSkipped');
    }
    if (answered) {
      return t('agent.tool.askAnswered');
    }
    return t('agent.tool.askWaiting');
  }

  function resolveSelected(questionId: string): string[] {
    const list = selectedIds[questionId];
    if (isNil(list)) {
      return [];
    }
    return list;
  }

  function resolveOtherText(questionId: string): string {
    const text = otherText[questionId];
    if (isNil(text)) {
      return '';
    }
    return text;
  }

  function resolveAnswerParts(question: RespAgentQuestionItem): string[] {
    const chosen = resolveSelected(question.id);
    const parts = question.options.filter(option => chosen.includes(option.id)).map(option => option.label);
    const other = resolveOtherText(question.id).trim();
    if (other.length > 0) {
      parts.push(other);
    }
    return parts;
  }

  function handlePickOption(question: RespAgentQuestionItem, optionId: string): void {
    setSelectedIds(prev => {
      const current = prev[question.id] ?? [];
      if (question.allow_multiple === true) {
        const next = current.includes(optionId) ? current.filter(id => id !== optionId) : [...current, optionId];
        return { ...prev, [question.id]: next };
      }
      return { ...prev, [question.id]: current.includes(optionId) ? [] : [optionId] };
    });
    if (question.allow_multiple !== true) {
      setOtherText(prev => ({ ...prev, [question.id]: '' }));
    }
  }

  function handleOtherTextChange(question: RespAgentQuestionItem, value: string): void {
    setOtherText(prev => ({ ...prev, [question.id]: value }));
    if (question.allow_multiple !== true && value.trim().length > 0) {
      setSelectedIds(prev => ({ ...prev, [question.id]: [] }));
    }
  }

  function questionAnswerReady(question: RespAgentQuestionItem): boolean {
    if (resolveSelected(question.id).length > 0) {
      return true;
    }
    return resolveOtherText(question.id).trim().length > 0;
  }

  function canContinue(): boolean {
    if (isNil(onAskAnswer) || questions.length === 0) {
      return false;
    }
    return questions.every(questionAnswerReady);
  }

  function buildAnswerText(questionId: string): string | null {
    const other = resolveOtherText(questionId).trim();
    if (other.length === 0) {
      return null;
    }
    return other;
  }

  function buildAnswers(): AgentQuestionAnswerItem[] {
    return questions.map(question => ({ questionId: question.id, optionIds: resolveSelected(question.id), text: buildAnswerText(question.id) }));
  }

  function handleContinue(): void {
    const callId = callIdRef.current;
    if (!canContinue() || isNil(onAskAnswer) || isNil(callId)) {
      return;
    }
    onAskAnswer({ callId, skipped: false, answers: buildAnswers() });
    setAnswered(true);
  }

  function handleSkip(): void {
    const callId = callIdRef.current;
    if (!isNil(onAskAnswer) && !isNil(callId)) {
      onAskAnswer({ callId, skipped: true, answers: [] });
    }
    setSkipped(true);
  }

  function applyStoredAnswers(answers: AgentQuestionAnswerItem[]): void {
    const nextSelected: Record<string, string[]> = {};
    const nextOther: Record<string, string> = {};
    for (const answer of answers) {
      nextSelected[answer.questionId] = answer.optionIds;
      if (!isNil(answer.text)) {
        nextOther[answer.questionId] = answer.text;
      }
    }
    setSelectedIds(nextSelected);
    setOtherText(nextOther);
  }

  function applyStoredState(state: string | undefined, answers: AgentQuestionAnswerItem[] | undefined): void {
    if (state === ASK_STATE.ANSWERED) {
      if (Array.isArray(answers)) {
        applyStoredAnswers(answers);
      }
      setAnswered(true);
      return;
    }
    if (state === ASK_STATE.SKIPPED) {
      setSkipped(true);
      return;
    }
    if (state === ASK_STATE.DECLINED) {
      setExpired(true);
    }
  }

  function onEvent(eventItem: AiChatItem): void {
    if (eventItem.type === RESP_EVENT_AGENT.ON_QUESTION) {
      callIdRef.current = eventItem.context.callId;
      setQuestions(Array.isArray(eventItem.context.questions) ? eventItem.context.questions : []);
      applyStoredState(eventItem.context.state, eventItem.context.answers);
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_QUESTION_ANSWERED && eventItem.context.callId === callIdRef.current) {
      applyStoredState(eventItem.context.state, eventItem.context.answers);
    }
  }

  function renderOption(question: RespAgentQuestionItem, optionId: string, label: string, letter: string, selected: boolean): ReactNode {
    return (
      <ButtonBase key={optionId} disableRipple className={selected ? 'tm-agent-ask-option is-selected' : 'tm-agent-ask-option'} onClick={() => handlePickOption(question, optionId)}>
        <Box component="span" className="tm-agent-ask-option-letter">
          {letter}
        </Box>
        <Box component="span" className="tm-agent-ask-option-label">
          {label}
        </Box>
        {selected ? <Check className="tm-agent-ask-option-check" /> : null}
      </ButtonBase>
    );
  }

  function renderOther(question: RespAgentQuestionItem): ReactNode {
    const filled = resolveOtherText(question.id).trim().length > 0;
    return (
      <Box className={filled ? 'tm-agent-ask-option-other is-filled' : 'tm-agent-ask-option-other'}>
        <Box component="span" className="tm-agent-ask-option-letter">
          {optionLetter(question.options.length)}
        </Box>
        <InputBase className="tm-agent-ask-other-field" placeholder={t('agent.tool.askOther')} value={resolveOtherText(question.id)} onChange={e => handleOtherTextChange(question, e.target.value)} />
      </Box>
    );
  }

  function renderQuestionForm(question: RespAgentQuestionItem): ReactNode {
    const chosen = resolveSelected(question.id);
    return (
      <Box key={question.id} className="tm-agent-ask-q">
        <Box className="tm-agent-ask-q-prompt">
          {question.prompt}
          {question.allow_multiple === true ? (
            <Box component="span" className="tm-agent-ask-q-multi">
              {t('agent.tool.askMultiHint')}
            </Box>
          ) : null}
        </Box>
        <Box className="tm-agent-ask-options">
          {question.options.map((option, optionIndex) => renderOption(question, option.id, option.label, optionLetter(optionIndex), chosen.includes(option.id)))}
          {renderOther(question)}
        </Box>
      </Box>
    );
  }

  function renderAnswerSummary(): ReactNode {
    if (!answered) {
      return null;
    }
    return (
      <Box className="tm-agent-ask-summary">
        {questions.map(question => (
          <Box key={question.id} className="tm-agent-ask-summary-row">
            <Box component="span" className="tm-agent-ask-summary-q">
              {question.prompt}
            </Box>
            <Box component="span" className="tm-agent-ask-summary-a">
              {resolveAnswerParts(question).join(t('common.listSeparator'))}
            </Box>
          </Box>
        ))}
      </Box>
    );
  }

  useItemHandle(ref, onEvent);

  useEffect(() => {
    if (!interactive) {
      return;
    }
    function onKeyDown(event: KeyboardEvent): void {
      if (event.key !== 'Escape' || event.defaultPrevented || questions.length === 0) {
        return;
      }
      const activeId = tabManagerService.getActiveTabId();
      const tab = activeId ? tabManagerService.getTab(activeId) : null;
      if (!tab || !tab.panes.some(pane => pane.id === paneId) || (tab.focusedPaneId && tab.focusedPaneId !== paneId)) {
        return;
      }
      if (document.querySelector('[role="menu"], [role="dialog"], [role="listbox"]')) {
        return;
      }
      event.preventDefault();
      handleSkip();
    }
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [interactive, questions, selectedIds, otherText, paneId, tabManagerService, onAskAnswer]);

  if (questions.length === 0) {
    return null;
  }

  if (interactive) {
    return (
      <Box className="tm-agent-tool-card tm-agent-ask-card">
        <Box className="tm-agent-tool-head">
          <Box className="tm-agent-tool-title">{resolveTitle()}</Box>
        </Box>
        <Box className="tm-agent-ask-body">{questions.map(renderQuestionForm)}</Box>
        <Box className="tm-agent-ask-actions">
          <ButtonBase disableRipple className="tm-agent-ask-skip" onClick={handleSkip}>
            <Box component="span">{t('agent.tool.askSkip')}</Box>
            <Box component="span" className="tm-agent-ask-kbd">
              Esc
            </Box>
          </ButtonBase>
          <Button disableRipple className="tm-btn tm-btn-primary tm-btn-compact" disabled={!canContinue()} onClick={handleContinue}>
            {t('agent.tool.askContinue')}
          </Button>
        </Box>
      </Box>
    );
  }

  return (
    <Box className="tm-agent-tool-card tm-agent-ask-card">
      <Box className="tm-agent-tool-head">
        <Box className="tm-agent-tool-title">{resolveTitle()}</Box>
      </Box>
      {renderAnswerSummary()}
    </Box>
  );
}
