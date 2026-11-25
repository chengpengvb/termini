import { useI18n } from '@/common/i18n/useI18n.ts';
import type { Terminal as XTerm } from '@xterm/xterm';
import { isNil, isNotNil } from '@/common/util/StringUtil.ts';
import type { ISearchOptions, SearchAddon } from '@xterm/addon-search';
import { Box, IconButton, InputBase, Tooltip, Typography } from '@mui/material';
import { Close, KeyboardArrowDown, KeyboardArrowUp } from '@mui/icons-material';
import { useEffect, useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type RefObject } from 'react';

export interface TerminalSearchProps {
  terminal: XTerm;
  getSearchAddon: () => SearchAddon | null;
  openRef?: RefObject<(() => void) | null>;
  registerKeyHandler: (handler: (event: KeyboardEvent) => boolean) => () => void;
}

function resolveSearchOptions(terminal: XTerm, caseSensitive: boolean, wholeWord: boolean): ISearchOptions {
  const css = getComputedStyle(terminal.element!);
  const matchBg = css.getPropertyValue('--tm-terminal-search-match-bg').trim();
  const matchBorder = css.getPropertyValue('--tm-terminal-search-match-border').trim();
  const activeBg = css.getPropertyValue('--tm-terminal-search-active-bg').trim();
  const activeBorder = css.getPropertyValue('--tm-terminal-search-active-border').trim();
  return {
    caseSensitive,
    wholeWord,
    decorations: {
      matchBorder,
      matchBackground: matchBg,
      activeMatchBorder: activeBorder,
      matchOverviewRuler: activeBorder,
      activeMatchBackground: activeBg,
      activeMatchColorOverviewRuler: activeBorder,
    },
  };
}

export function TerminalSearch(props: TerminalSearchProps) {
  const { t } = useI18n();
  const inputRef = useRef<HTMLInputElement>(null);
  const [term, setTerm] = useState<string | null>(null);
  const [visible, setVisible] = useState<boolean>(false);
  const [wholeWord, setWholeWord] = useState<boolean>(false);
  const [resultIndex, setResultIndex] = useState<number>(-1);
  const [resultCount, setResultCount] = useState<number>(0);
  const [focusTrigger, setFocusTrigger] = useState<number>(0);
  const [searchPending, setSearchPending] = useState<boolean>(false);
  const [caseSensitive, setCaseSensitive] = useState<boolean>(false);
  const { terminal, getSearchAddon, openRef, registerKeyHandler } = props;

  const handleOpen = (): void => {
    setVisible(true);
    setFocusTrigger(n => n + 1);
  };

  const handleClose = (): void => {
    setVisible(false);
    setSearchPending(false);
    getSearchAddon()?.clearDecorations();
    terminal.focus();
  };

  const handleFindNext = (): void => {
    const addon = getSearchAddon();
    if (!addon || isNil(term)) {
      return;
    }
    addon.findNext(term, resolveSearchOptions(terminal, caseSensitive, wholeWord));
  };

  const handleFindPrevious = (): void => {
    const addon = getSearchAddon();
    if (!addon || isNil(term)) {
      return;
    }
    addon.findPrevious(term, resolveSearchOptions(terminal, caseSensitive, wholeWord));
  };

  const handleToggleCase = (): void => {
    if (isNotNil(term)) {
      setSearchPending(true);
    }
    setCaseSensitive(v => !v);
  };

  const handleToggleWholeWord = (): void => {
    if (isNotNil(term)) {
      setSearchPending(true);
    }
    setWholeWord(v => !v);
  };

  const handleChange = (next: string): void => {
    if (next.length === 0) {
      setTerm(null);
      setSearchPending(false);
      return;
    }
    setSearchPending(true);
    setTerm(next);
  };

  const handleKeyDown = (event: ReactKeyboardEvent<HTMLInputElement>): void => {
    if (event.key === 'Escape') {
      event.preventDefault();
      handleClose();
      return;
    }
    if (event.key === 'Enter') {
      event.preventDefault();
      if (event.shiftKey) {
        handleFindPrevious();
        return;
      }
      handleFindNext();
    }
  };

  const resolveResultText = (): string => {
    if (isNil(term) || searchPending) {
      return '';
    }
    if (resultCount === 0) {
      return t('protocol.terminal.searchNoResult');
    }
    return `${resultIndex + 1}/${resultCount}`;
  };

  const resolveToggleClassName = (active: boolean): string => {
    return active ? 'tm-terminal-search-toggle is-active' : 'tm-terminal-search-toggle is-inactive';
  };

  useEffect(() => {
    if (!openRef) {
      return;
    }
    openRef.current = handleOpen;
    return () => {
      openRef.current = null;
    };
  });

  useEffect(() => {
    return registerKeyHandler((event: KeyboardEvent): boolean => {
      if (event.type !== 'keydown') {
        return true;
      }
      const isFind = (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'f';
      if (!isFind) {
        return true;
      }
      setVisible(true);
      setFocusTrigger(t => t + 1);
      return false;
    });
  }, [registerKeyHandler]);

  useEffect(() => {
    if (!visible) {
      return;
    }
    inputRef.current?.focus({preventScroll: true});
    inputRef.current?.select();
  }, [visible, focusTrigger]);

  useEffect(() => {
    const addon = getSearchAddon();
    if (!addon) {
      return;
    }
    const subscription = addon.onDidChangeResults(e => {
      setResultCount(e.resultCount);
      setResultIndex(e.resultIndex);
      setSearchPending(false);
    });
    return () => {
      subscription.dispose();
    };
  }, [terminal, getSearchAddon]);

  useEffect(() => {
    if (!visible) {
      return;
    }
    const addon = getSearchAddon();
    if (!addon) {
      return;
    }
    if (isNil(term)) {
      addon.clearDecorations();
      setResultCount(0);
      setResultIndex(-1);
      setSearchPending(false);
      return;
    }
    setSearchPending(true);
    addon.findNext(term, resolveSearchOptions(terminal, caseSensitive, wholeWord));
  }, [term, caseSensitive, wholeWord, visible]);

  if (!visible) {
    return null;
  }

  return (
    <Box className="tm-terminal-search" data-tm-gesture-ignore="true">
      <Box className="tm-terminal-search-input-wrap">
        <InputBase inputRef={inputRef} value={isNotNil(term) ? term : ''} placeholder={t('protocol.terminal.searchFind')} onChange={e => handleChange(e.target.value)} onKeyDown={handleKeyDown} className="tm-terminal-search-input" />
        {isNotNil(term) && <Typography className={resultCount === 0 && !searchPending ? 'tm-terminal-search-result is-empty' : 'tm-terminal-search-result'}>{resolveResultText()}</Typography>}
      </Box>
      <Tooltip title={t('protocol.terminal.searchCaseSensitive')}>
        <IconButton size="small" className={resolveToggleClassName(caseSensitive)} onClick={handleToggleCase}>
          Aa
        </IconButton>
      </Tooltip>
      <Tooltip title={t('protocol.terminal.searchWholeWord')}>
        <IconButton size="small" className={resolveToggleClassName(wholeWord)} onClick={handleToggleWholeWord}>
          ab
        </IconButton>
      </Tooltip>
      <Tooltip title={t('protocol.terminal.searchPrevious')}>
        <IconButton size="small" className="tm-terminal-search-nav-btn" onClick={handleFindPrevious}>
          <KeyboardArrowUp sx={{ fontSize: 16 }} />
        </IconButton>
      </Tooltip>
      <Tooltip title={t('protocol.terminal.searchNext')}>
        <IconButton size="small" className="tm-terminal-search-nav-btn" onClick={handleFindNext}>
          <KeyboardArrowDown sx={{ fontSize: 16 }} />
        </IconButton>
      </Tooltip>
      <Tooltip title={t('protocol.terminal.searchClose')}>
        <IconButton size="small" className="tm-terminal-search-nav-btn" onClick={handleClose}>
          <Close sx={{ fontSize: 16 }} />
        </IconButton>
      </Tooltip>
    </Box>
  );
}
