import { useI18n } from '@/common/i18n/useI18n.ts';
import { useEffect, useRef, useState } from 'react';
import type { KeyboardEvent as ReactKeyboardEvent } from 'react';
import { useSnippetService } from '@/common/context/useAppHooks.tsx';
import { Box, Button, ButtonBase, InputBase, Typography } from '@mui/material';
import { matchSnippet, renderSnippetCommand, snippetVariables, type Snippet } from '@/common/model/Snippet.ts';

export interface TerminalSnippetPanelProps {
  onInput: (data: string) => void;
}

export function TerminalSnippetPanel(props: TerminalSnippetPanelProps) {
  const { t } = useI18n();
  const { onInput } = props;
  const snippetService = useSnippetService();
  const [run, setRun] = useState<boolean>(true);
  const [query, setQuery] = useState<string>('');
  const keyword = query.trim().toLowerCase();
  const inputRef = useRef<HTMLInputElement | null>(null);
  const [snippets, setSnippets] = useState<Snippet[]>([]);
  const [pending, setPending] = useState<Snippet | null>(null);
  const [values, setValues] = useState<Record<string, string>>({});
  const filtered = keyword ? snippets.filter(item => matchSnippet(item, keyword)) : snippets;
  const variables = pending ? snippetVariables(pending.command) : [];

  const send = (command: string, isRun: boolean): void => {
    onInput(isRun ? `${command}\r` : command);
    setPending(null);
    setValues({});
  };

  const handleSend = (snippet: Snippet, isRun: boolean): void => {
    const names = snippetVariables(snippet.command);
    if (names.length === 0) {
      send(snippet.command, isRun);
      return;
    }
    setRun(isRun);
    setPending(snippet);
    setValues(Object.fromEntries(names.map(name => [name, ''])));
  };

  const handleCancel = (): void => {
    setPending(null);
    setValues({});
  };

  const handleSubmit = (): void => {
    if (!pending) {
      return;
    }
    send(renderSnippetCommand(pending.command, values), run);
  };

  const handleVariableKeyDown = (event: ReactKeyboardEvent<HTMLInputElement>): void => {
    if (event.key === 'Escape') {
      event.preventDefault();
      handleCancel();
      return;
    }
    if (event.key === 'Enter') {
      event.preventDefault();
      handleSubmit();
    }
  };

  useEffect(() => {
    void snippetService.selectAll().then(list => {
      setSnippets([...list].sort((a, b) => a.sort - b.sort));
    });
  }, [snippetService]);

  useEffect(() => {
    if (!pending) {
      return;
    }
    inputRef.current?.focus({ preventScroll: true });
  }, [pending]);

  if (pending) {
    return (
      <Box className="tm-terminal-snippet-panel">
        <Box className="tm-terminal-snippet-panel-pending">
          <Typography className="tm-terminal-snippet-panel-pending-name">{pending.name}</Typography>
          <Typography className="tm-terminal-snippet-preview">{renderSnippetCommand(pending.command, values)}</Typography>
          {variables.map((name, index) => (
            <Box key={name} className="tm-terminal-snippet-variable">
              <Typography className="tm-terminal-snippet-variable-name">{name}</Typography>
              <InputBase
                inputRef={index === 0 ? inputRef : undefined}
                className="tm-terminal-snippet-input"
                value={values[name]}
                placeholder={t('protocol.terminal.snippetVariablePlaceholder')}
                onChange={event => setValues({ ...values, [name]: event.target.value })}
                onKeyDown={handleVariableKeyDown}
              />
            </Box>
          ))}
          <Box className="tm-terminal-snippet-foot">
            <Button disableRipple className="tm-btn tm-btn-secondary" onClick={handleCancel}>
              {t('common.cancel')}
            </Button>
            <Button disableRipple className="tm-btn tm-btn-primary" onClick={handleSubmit}>
              {run ? t('protocol.terminal.snippetRun') : t('protocol.terminal.snippetPaste')}
            </Button>
          </Box>
        </Box>
      </Box>
    );
  }

  return (
    <Box className="tm-terminal-snippet-panel">
      <InputBase value={query} className="tm-terminal-snippet-input" placeholder={t('home.snippet.searchPlaceholder')} onChange={event => setQuery(event.target.value)} />
      <Box className="tm-terminal-snippet-panel-list">
        {filtered.length === 0 ? (
          <Typography className="tm-terminal-snippet-panel-empty">{t('protocol.terminal.snippetEmpty')}</Typography>
        ) : (
          filtered.map(item => (
            <Box key={item.id} className="tm-terminal-snippet-panel-row">
              <Box className="tm-terminal-snippet-panel-row-head">
                <Typography className="tm-terminal-snippet-panel-row-name">{item.name}</Typography>
                <Box className="tm-terminal-snippet-panel-row-actions">
                  <ButtonBase disableRipple className="tm-terminal-snippet-panel-action" onClick={() => handleSend(item, true)}>
                    {t('protocol.terminal.snippetRun')}
                  </ButtonBase>
                  <ButtonBase disableRipple className="tm-terminal-snippet-panel-action" onClick={() => handleSend(item, false)}>
                    {t('protocol.terminal.snippetPaste')}
                  </ButtonBase>
                </Box>
              </Box>
              <Typography className="tm-terminal-snippet-panel-row-command">{item.command}</Typography>
            </Box>
          ))
        )}
      </Box>
    </Box>
  );
}
