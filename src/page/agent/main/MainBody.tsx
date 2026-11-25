import { Box } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { Ref, useImperativeHandle, useRef, useState } from 'react';
import type { ChatReference } from '@/common/model/dto/ChatReference.ts';
import { MessageBody } from '@/page/agent/main/body/message/MessageBody.tsx';
import type { AgentQuestionAnswer } from '@/common/model/req/event/WsAgent.ts';
import { InputBottom, type InputBottomHandle } from '@/page/agent/main/input/InputBottom.tsx';

export interface MainBodyHandle {
  applyReference: (ref: ChatReference) => void;
}

export interface MainBodyProps {
  sessionId: string;
  chatId: string | null;
  ref?: Ref<MainBodyHandle>;
  setChatId: (chatId: string | null) => void;
}

export function MainBody(props: MainBodyProps) {
  const { t } = useI18n();
  const inputRef = useRef<InputBottomHandle>(null);
  const [reloadTick, setReloadTick] = useState<number>(0);
  const { chatId, setChatId, sessionId, ref } = props;
  const [revertItemId, setRevertItemId] = useState<string | null>(null);

  function resolveGreeting(): string {
    const hour = new Date().getHours();
    if (hour < 6) {
      return t('agent.empty.greetingLate');
    }
    if (hour < 12) {
      return t('agent.empty.greetingMorning');
    }
    if (hour < 18) {
      return t('agent.empty.greetingAfternoon');
    }
    return t('agent.empty.greetingEvening');
  }

  function applyReference(chatRef: ChatReference): void {
    const handle = inputRef.current;
    if (!handle) {
      return;
    }
    handle.applyReference(chatRef);
  }

  function handleRevertEdit(itemId: string, message: string): void {
    const handle = inputRef.current;
    if (!handle) {
      return;
    }
    handle.beginRevert(itemId, message);
  }

  function handleAskAnswer(answer: AgentQuestionAnswer): void {
    const handle = inputRef.current;
    if (!handle) {
      return;
    }
    handle.answerQuestion(answer);
  }

  function handleReverted(): void {
    setReloadTick((current: number) => current + 1);
  }

  function handleInterrupted(message: string): void {
    inputRef.current?.restoreDraft(message);
  }

  useImperativeHandle(ref, () => ({ applyReference }));

  if (chatId) {
    return (
      <Box className="tm-agent-main">
        <MessageBody key={`${chatId}:${reloadTick}`} sessionId={sessionId} chatId={chatId} revertItemId={revertItemId} onAskAnswer={handleAskAnswer} onRevertEdit={handleRevertEdit} onInterruptedWithoutOutput={handleInterrupted} />
        <InputBottom ref={inputRef} sessionId={sessionId} chatId={chatId} setChatId={setChatId} onReverted={handleReverted} onRevertChange={setRevertItemId} />
      </Box>
    );
  }

  return (
    <Box className="tm-agent-main is-empty">
      <Box className="tm-agent-empty">
        <Box className="tm-agent-empty-hero">
          <Box className="tm-agent-empty-title">{resolveGreeting()}</Box>
        </Box>
        <InputBottom ref={inputRef} sessionId={sessionId} chatId={chatId} setChatId={setChatId} onReverted={handleReverted} onRevertChange={setRevertItemId} />
      </Box>
    </Box>
  );
}
