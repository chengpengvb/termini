import { useI18n } from '@/common/i18n/useI18n.ts';
import { useEffect, useRef, useState } from 'react';
import type { Terminal as XTerm } from '@xterm/xterm';
import type { KeyboardEvent as ReactKeyboardEvent } from 'react';
import { useSnippetService } from '@/common/context/useAppHooks.tsx';
import { Box, Button, ButtonBase, InputBase, Typography } from '@mui/material';
import { matchSnippet, renderSnippetCommand, snippetVariables, type Snippet } from '@/common/model/Snippet.ts';

export interface TerminalSnippetProps {
  terminal: XTerm;
  onInput: (data: string) => void;
  registerKeyHandler: (handler: (event: KeyboardEvent) => boolean) => () => void;
}

export function TerminalSnippet(props: TerminalSnippetProps) {
  const { t } = useI18n();
  const snippetService = useSnippetService();
  const [query, setQuery] = useState<string>('');
  const keyword = query.trim().toLowerCase();
  const [active, setActive] = useState<number>(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const { terminal, onInput, registerKeyHandler } = props;
  const [visible, setVisible] = useState<boolean>(false);
  const [snippets, setSnippets] = useState<Snippet[]>([]);
  const [pending, setPending] = useState<Snippet | null>(null);
  const variables = pending ? snippetVariables(pending.command) : [];
  const [values, setValues] = useState<Record<string, string>>({});
  const filtered = keyword ? snippets.filter(item => matchSnippet(item, keyword)) : snippets;

  const handleClose = (): void => {
    setVisible(false);
    setPending(null);
    setValues({});
    terminal.focus();
  };

  const send = (command: string): void => {
    onInput(`${command}\r`);
    handleClose();
  };

  const handlePick = (snippet: Snippet): void => {
    const names = snippetVariables(snippet.command);
    if (names.length === 0) {
      send(snippet.command);
      return;
    }
    setPending(snippet);
    setValues(Object.fromEntries(names.map(name => [name, ''])));
  };

  const handleSubmitVariables = (): void => {
    if (!pending) {
      return;
    }
    send(renderSnippetCommand(pending.command, values));
  };

  const handleListKeyDown = (event: ReactKeyboardEvent<HTMLInputElement>): void => {
    if (event.key === 'Escape') {
      event.preventDefault();
      handleClose();
      return;
    }
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      setActive(prev => (filtered.length === 0 ? 0 : (prev + 1) % filtered.length));
      return;
    }
    if (event.key === 'ArrowUp') {
      event.preventDefault();
      setActive(prev => (filtered.length === 0 ? 0 : (prev - 1 + filtered.length) % filtered.length));
      return;
    }
    if (event.key === 'Enter') {
      event.preventDefault();
      const picked = filtered[active];
      if (!picked) {
        return;
      }
      handlePick(picked);
    }
  };

  const handleVariableKeyDown = (event: ReactKeyboardEvent<HTMLInputElement>): void => {
    if (event.key === 'Escape') {
      event.preventDefault();
      handleClose();
      return;
    }
    if (event.key === 'Enter') {
      event.preventDefault();
      handleSubmitVariables();
    }
  };

  useEffect(() => {
    return registerKeyHandler((event: KeyboardEvent): boolean => {
      if (event.type !== 'keydown') {
        return true;
      }
      const isSnippet = (event.ctrlKey || event.metaKey) && event.shiftKey && event.key.toLowerCase() === 'p';
      if (!isSnippet) {
        return true;
      }
      setVisible(true);
      setQuery('');
      setActive(0);
      setPending(null);
      return false;
    });
  }, [registerKeyHandler]);

  useEffect(() => {
    if (!visible) {
      return;
    }
    inputRef.current?.focus({ preventScroll: true });
    void snippetService.selectAll().then(list => {
      setSnippets([...list].sort((a, b) => a.sort - b.sort));
    });
  }, [visible, snippetService]);

  useEffect(() => {
    if (!visible || !pending) {
      return;
    }
    inputRef.current?.focus({ preventScroll: true });
  }, [visible, pending]);

  if (!visible) {
    return null;
  }

  if (pending) {
    return (
      <Box className="tm-terminal-snippet">
        <Typography className="tm-terminal-snippet-title">{pending.name}</Typography>
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
          <Button disableRipple className="tm-btn tm-btn-secondary" onClick={handleClose}>
            {t('common.cancel')}
          </Button>
          <Button disableRipple className="tm-btn tm-btn-primary" onClick={handleSubmitVariables}>
            {t('protocol.terminal.snippetSend')}
          </Button>
        </Box>
      </Box>
    );
  }

  return (
    <Box className="tm-terminal-snippet">
      <InputBase
        inputRef={inputRef}
        value={query}
        className="tm-terminal-snippet-input"
        placeholder={t('protocol.terminal.snippetSearch')}
        onChange={event => {
          setQuery(event.target.value);
          setActive(0);
        }}
        onKeyDown={handleListKeyDown}
      />
      <Box className="tm-terminal-snippet-list">
        {filtered.length === 0 ? (
          <Typography className="tm-terminal-snippet-empty">{t('protocol.terminal.snippetEmpty')}</Typography>
        ) : (
          filtered.map((item, index) => (
            <ButtonBase key={item.id} disableRipple className={index === active ? 'tm-terminal-snippet-row is-active' : 'tm-terminal-snippet-row'} onMouseEnter={() => setActive(index)} onClick={() => handlePick(item)}>
              <Typography className="tm-terminal-snippet-row-name">{item.name}</Typography>
              <Typography className="tm-terminal-snippet-row-command">{item.command}</Typography>
            </ButtonBase>
          ))
        )}
      </Box>
    </Box>
  );
}
