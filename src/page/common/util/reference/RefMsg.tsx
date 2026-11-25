import { Box } from '@mui/material';
import { CHAT_TARGET_TYPE } from '@/common/const/ChatTargetConst.ts';
import { ChatTextFold } from '@/page/common/util/reference/ChatTextFold.tsx';
import { ReferenceChip } from '@/page/common/util/reference/ReferenceChip.tsx';
import { useEffect, useRef, type ClipboardEvent, type ReactNode } from 'react';
import { isChatTextPath, MESSAGE_PART, splitMessageParts } from '@/page/common/util/reference/token.ts';

interface RefMsgProps {
  text: string;
  foldChatText?: boolean;
  foldedPaths?: string[];
}

export function RefMsg(props: RefMsgProps): ReactNode {
  const { text } = props;
  const foldedPaths = props.foldedPaths;
  const parts = splitMessageParts(text);
  const rootRef = useRef<HTMLSpanElement>(null);
  const foldChatText = props.foldChatText === true;

  function resolveFoldedPaths(): string[] {
    if (foldedPaths === undefined) {
      return [];
    }
    return foldedPaths;
  }

  function shouldFoldLocalPath(path: string): boolean {
    if (!foldChatText) {
      return false;
    }
    if (!isChatTextPath(path)) {
      return false;
    }
    return !resolveFoldedPaths().includes(path);
  }

  function handleCopy(event: ClipboardEvent<HTMLElement>): void {
    event.preventDefault();
    event.clipboardData.setData('text/plain', text);
  }

  function markSelectedChips(): void {
    const root = rootRef.current;
    if (root === null) {
      return;
    }
    const sel = window.getSelection();
    if (sel === null) {
      throw new Error('window.getSelection() is null');
    }
    if (sel.rangeCount === 0 || sel.isCollapsed) {
      root.querySelectorAll('[data-reference="1"]').forEach(el => {
        el.classList.remove('is-sel');
      });
      return;
    }
    const range = sel.getRangeAt(0);
    root.querySelectorAll('[data-reference="1"]').forEach(el => {
      el.classList.toggle('is-sel', range.intersectsNode(el));
    });
  }

  useEffect(() => {
    document.addEventListener('selectionchange', markSelectedChips);
    return () => {
      document.removeEventListener('selectionchange', markSelectedChips);
    };
  }, []);

  return (
    <Box ref={rootRef} component="span" className="tm-agent-reference-message" onCopy={handleCopy}>
      {parts.map((part, index) => {
        if (part.part === MESSAGE_PART.TEXT) {
          return (
            <Box component="span" key={`text-${index}`} className="tm-agent-reference-message-text">
              {part.text}
            </Box>
          );
        }
        if (part.token.kind === CHAT_TARGET_TYPE.LOCAL_PATH && shouldFoldLocalPath(part.token.id)) {
          return <ChatTextFold key={`fold-${part.token.kind}-${part.token.id}-${index}`} path={part.token.id} foldedPaths={resolveFoldedPaths()} renderContent={(innerText, nextPaths) => <RefMsg text={innerText} foldChatText={true} foldedPaths={nextPaths} />} />;
        }
        return <ReferenceChip key={`ref-${part.token.kind}-${part.token.id}-${index}`} kind={part.token.kind} id={part.token.id} interactive={false} onRemove={null} />;
      })}
    </Box>
  );
}
