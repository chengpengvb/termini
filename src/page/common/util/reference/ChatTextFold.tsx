import { Box } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { isNil, isNotNil } from '@/common/util/StringUtil.ts';
import { useFileEditService } from '@/common/context/useAppHooks.tsx';
import { useEffect, useState, type MouseEvent, type PointerEvent as ReactPointerEvent, type ReactNode } from 'react';

interface ChatTextFoldProps {
  path: string;
  foldedPaths: string[];
  renderContent: (text: string, nextPaths: string[]) => ReactNode;
}

export function ChatTextFold(props: ChatTextFoldProps): ReactNode {
  const { t, tf } = useI18n();
  const [open, setOpen] = useState(false);
  const fileEditService = useFileEditService();
  const { path, foldedPaths, renderContent } = props;
  const [error, setError] = useState<string | null>(null);
  const [content, setContent] = useState<string | null>(null);

  function resolveLineCount(): number {
    if (isNil(content)) {
      return 0;
    }
    return content.split('\n').length;
  }

  function resolveClassName(): string {
    if (open) {
      return 'tm-agent-paste-fold is-open';
    }
    return 'tm-agent-paste-fold';
  }

  function handlePointerDown(event: ReactPointerEvent<HTMLElement>): void {
    event.stopPropagation();
  }

  function handleToggle(event: MouseEvent<HTMLElement>): void {
    event.stopPropagation();
    const target = event.target;
    if (target instanceof Element && target.closest('[data-reference="1"]')) {
      return;
    }
    const sel = window.getSelection();
    if (sel !== null && sel.rangeCount > 0 && !sel.isCollapsed) {
      return;
    }
    setOpen((current: boolean) => !current);
  }

  useEffect(() => {
    let cancelled = false;
    fileEditService
      .read(path)
      .then(data => {
        if (cancelled) {
          return;
        }
        setContent(data.content);
        setError(null);
      })
      .catch((caught: unknown) => {
        if (cancelled) {
          return;
        }
        if (caught instanceof Error) {
          setError(caught.message);
          return;
        }
        throw caught;
      });
    return () => {
      cancelled = true;
    };
  }, [path, fileEditService]);

  const lineCount = resolveLineCount();
  const nextPaths = [...foldedPaths, path];

  return (
    <Box component="span" className={resolveClassName()} onClick={handleToggle} onPointerDown={handlePointerDown}>
      {isNotNil(error) ? (
        <Box component="span" className="tm-agent-paste-fold-error">
          {error}
        </Box>
      ) : isNil(content) ? (
        <Box component="span" className="tm-agent-paste-fold-preview tm-agent-shimmer">
          {t('agent.message.loading')}
        </Box>
      ) : (
        <Box component="span" className="tm-agent-paste-fold-preview">
          {renderContent(content, nextPaths)}
        </Box>
      )}
      {lineCount > 0 ? (
        <Box component="span" className="tm-agent-paste-fold-meta">
          {tf('agent.message.chatTextLines', { n: lineCount })}
        </Box>
      ) : null}
    </Box>
  );
}
