import type { RefObject } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import type { Terminal as XTerm } from '@xterm/xterm';
import { usePaneContext } from '@/common/components/tab';
import { Box, ButtonBase, IconButton } from '@mui/material';
import { useCallback, useEffect, useRef, useState } from 'react';
import { TerminalSnippetPanel } from './TerminalSnippetPanel.tsx';
import { ArrowForward, DataObject, Tune } from '@mui/icons-material';
import { ThemeField } from '@/page/common/components/input/ThemeField.tsx';
import { SelectField } from '@/page/common/components/input/SelectField.tsx';
import { Section } from '@/page/home/devices/edit/section/shared/Section.tsx';
import { StepperField } from '@/page/common/components/input/StepperField.tsx';
import { useConfigService, useTerminalThemeService } from '@/common/context/useAppHooks.tsx';
import {
  applyTerminalCursorStyle,
  remToPx,
  resolveTerminalCursorOptions,
  resolveTerminalFontOptions,
  resolveTerminalFontStack,
  resolveTerminalTheme,
  resolveTerminalThemeColors,
  TERMINAL_FONT_SIZE_MAX_REM,
  TERMINAL_FONT_SIZE_MIN_REM,
  TERMINAL_FONT_SIZE_STEP_REM,
  TerminalSetting,
} from '@/page/common/util/terminal.ts';

const PANEL_TAB = {
  SNIPPETS: 1,
  SETTINGS: 2,
} as const;

export interface TerminalSettingsPanelProps {
  fontRevision?: number;
  settingKey: string;
  onClose: () => void;
  onRefit?: () => void;
  terminal: XTerm | null;
  onInput: (data: string) => void;
  hostRef: RefObject<HTMLElement | null>;
}

export function TerminalSettingsPanel(props: TerminalSettingsPanelProps) {
  const { t } = useI18n();
  const { paneId } = usePaneContext();
  const configService = useConfigService();
  const appliedThemeRef = useRef<string | null>(null);
  const terminalThemeService = useTerminalThemeService();
  const [tab, setTab] = useState<number>(PANEL_TAB.SETTINGS);
  const { terminal, settingKey, hostRef, onInput, onClose, onRefit } = props;
  const [setting, setSetting] = useState<TerminalSetting>(() => configService.selectTerminalSetting(settingKey));
  const { fontFamily, fontSize, themeId, cursorStyle } = setting;
  useEffect(() => {
    setSetting(configService.selectTerminalSetting(settingKey));
  }, [props.fontRevision, settingKey, configService]);

  const updateSetting = useCallback(
    (patch: Partial<TerminalSetting>) => {
      setSetting(prev => {
        const next = { ...prev, ...patch };
        configService.saveTerminalSetting(settingKey, next);
        return next;
      });
    },
    [configService, settingKey],
  );

  useEffect(() => {
    if (!terminal) {
      return;
    }
    terminal.options.fontFamily = resolveTerminalFontStack(fontFamily);
    onRefit?.();
    document.fonts.ready.then(() => onRefit?.());
  }, [terminal, fontFamily]);

  useEffect(() => {
    if (!terminal) {
      return;
    }
    terminal.options.fontSize = remToPx(fontSize);
    onRefit?.();
  }, [terminal, fontSize]);

  useEffect(() => {
    if (!terminal) {
      return;
    }
    applyTerminalCursorStyle(terminal, cursorStyle);
  }, [terminal, cursorStyle]);

  const applyPaneTheme = useCallback(() => {
    const host = hostRef.current;
    if (!host || !terminal) {
      return;
    }
    const paneThemeId = terminalThemeService.getPaneThemeId(paneId);
    if (appliedThemeRef.current === paneThemeId) {
      return;
    }
    const shell = host.closest('.tm-terminal-shell');
    if (!shell) {
      console.warn('[terminal] terminal shell missing while applying theme');
      return;
    }
    shell.setAttribute('data-terminal-theme', paneThemeId);
    appliedThemeRef.current = paneThemeId;
    terminal.options.theme = {
      ...resolveTerminalTheme(),
      ...resolveTerminalThemeColors(paneThemeId, host),
    };
    terminal.refresh(0, terminal.rows - 1);
  }, [terminal, hostRef, paneId, terminalThemeService]);

  useEffect(() => {
    terminalThemeService.registerPaneTheme(paneId, themeId);
  }, [themeId, paneId, terminalThemeService]);

  useEffect(() => {
    applyPaneTheme();
    return terminalThemeService.addChangeListener(applyPaneTheme);
  }, [applyPaneTheme, terminalThemeService]);

  return (
    <Box className="tm-terminal-settings-panel">
      <Box className="tm-terminal-settings-header">
        <Box className="tm-terminal-settings-tabs">
          <ButtonBase aria-label={t('home.menu.snippets')} onClick={() => setTab(PANEL_TAB.SNIPPETS)} className={tab === PANEL_TAB.SNIPPETS ? 'tm-terminal-settings-tab is-active' : 'tm-terminal-settings-tab'}>
            <DataObject />
          </ButtonBase>
          <ButtonBase aria-label={t('protocol.terminal.settings')} onClick={() => setTab(PANEL_TAB.SETTINGS)} className={tab === PANEL_TAB.SETTINGS ? 'tm-terminal-settings-tab is-active' : 'tm-terminal-settings-tab'}>
            <Tune />
          </ButtonBase>
        </Box>
        <IconButton size="small" className="tm-terminal-settings-close" onClick={onClose} aria-label={t('common.foldPanel')}>
          <ArrowForward fontSize="small" />
        </IconButton>
      </Box>
      {tab === PANEL_TAB.SNIPPETS ? (
        <TerminalSnippetPanel onInput={onInput} />
      ) : (
        <Box className="tm-terminal-settings-body">
          <SelectField label={t('protocol.terminal.font')} value={fontFamily} onChange={value => updateSetting({ fontFamily: value })} options={resolveTerminalFontOptions()} />
          <StepperField label={t('protocol.terminal.fontSize')} value={fontSize} onChange={value => updateSetting({ fontSize: value })} min={TERMINAL_FONT_SIZE_MIN_REM} max={TERMINAL_FONT_SIZE_MAX_REM} step={TERMINAL_FONT_SIZE_STEP_REM} suffix="rem" />
          <SelectField label={t('protocol.terminal.cursor')} value={cursorStyle} onChange={value => updateSetting({ cursorStyle: value })} options={resolveTerminalCursorOptions()} />
          <Section title={t('protocol.terminal.theme')}>
            <ThemeField value={themeId} onChange={value => updateSetting({ themeId: value })} />
          </Section>
        </Box>
      )}
    </Box>
  );
}
