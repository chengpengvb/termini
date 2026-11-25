import { useI18n } from '@/common/i18n/useI18n.ts';
import ArrowDropUp from '@mui/icons-material/ArrowDropUp';
import { Box, IconButton, Typography } from '@mui/material';
import ArrowDropDown from '@mui/icons-material/ArrowDropDown';
import { StepEvent } from '@/page/common/dto/terminal/StepEvent.ts';
import { LOG_LEVEL, LogEntry } from '@/page/common/dto/terminal/LogEntry.ts';
import { LogEntryItem } from '@/page/protocol/common/step/log/LogEntryItem.tsx';
import { LogHeaderLatest } from '@/page/protocol/common/step/log/LogHeaderLatest.tsx';
import { type Ref, useEffect, useImperativeHandle, useReducer, useRef, useState } from 'react';

export interface StepLogHandle {
  clear: () => void;
  pushStepEvent: (stepEvent: StepEvent) => void;
}

export interface StepLogProps {
  failed?: boolean;
  ref?: Ref<StepLogHandle>;
  setOnOpened: (value: boolean) => void;
}

export function StepLog(props: StepLogProps) {
  const { t } = useI18n();
  const logIdRef = useRef<number>(0);
  const logsRef = useRef<LogEntry[]>([]);
  const { failed, ref, setOnOpened } = props;
  const bodyRef = useRef<HTMLDivElement | null>(null);
  const [expanded, setExpanded] = useState<boolean>(false);
  const [renderTick, forceUpdate] = useReducer((count: number) => count + 1, 0);

  const appendLog = (level: number, text: string): void => {
    logIdRef.current += 1;
    const entry: LogEntry = { id: logIdRef.current, level: level, text: text, timestamp: Date.now() };
    logsRef.current = [...logsRef.current, entry];
    forceUpdate();
  };

  const resolveLogLevel = (stepEvent: StepEvent): number => {
    if (stepEvent.stop && stepEvent.error) {
      return LOG_LEVEL.ERROR;
    }
    return LOG_LEVEL.INFO;
  };

  const resolveDataLogLevel = (stepEvent: StepEvent): number => {
    if (stepEvent.error) {
      return LOG_LEVEL.ERROR;
    }
    return LOG_LEVEL.INFO;
  };

  const notifyStepEventOpened = (next: StepEvent): void => {
    if (!next.opened) {
      return;
    }
    setOnOpened(true);
  };

  const pushStepEvent = (next: StepEvent): void => {
    const message = next.stepTypeMessage;
    if (next.error) {
      setExpanded(true);
    }
    if (message) {
      appendLog(resolveLogLevel(next), message);
    }
    const data = next.data;
    if (data) {
      appendLog(resolveDataLogLevel(next), String(data));
    }
    notifyStepEventOpened(next);
  };

  const toggleExpanded = (): void => {
    setExpanded(prev => !prev);
  };

  const clear = (): void => {
    forceUpdate();
    logsRef.current = [];
    logIdRef.current = 0;
    setExpanded(false);
  };

  const resolvePanelClass = (): string => {
    if (expanded) {
      return 'tm-protocol-log-panel is-expanded';
    }
    return 'tm-protocol-log-panel';
  };

  useEffect(() => {
    if (!expanded) {
      return;
    }
    const el = bodyRef.current;
    if (!el) {
      return;
    }
    el.scrollTop = el.scrollHeight;
  }, [expanded, renderTick]);

  useImperativeHandle(ref, () => ({ clear, pushStepEvent }));

  return (
    <Box className="tm-protocol-log">
      {failed && <Typography className="tm-protocol-failure-title">{t('protocol.log.failureTitle')}</Typography>}
      <Box className={resolvePanelClass()}>
        <Box className="tm-protocol-log-header">
          <Typography className="tm-protocol-log-header-title">{t('protocol.log.title')}</Typography>
          {!expanded && <LogHeaderLatest logs={logsRef.current} />}
          <Box className="tm-protocol-log-header-actions">
            <IconButton className="tm-protocol-log-icon-btn" onClick={toggleExpanded} title={expanded ? t('common.foldPanel') : t('common.expandPanel')}>
              {expanded ? <ArrowDropUp /> : <ArrowDropDown />}
            </IconButton>
          </Box>
        </Box>
        {expanded && (
          <Box ref={bodyRef} className="tm-protocol-log-body">
            {logsRef.current.length === 0 && <Typography className="tm-protocol-log-empty">{t('protocol.log.empty')}</Typography>}
            {logsRef.current.map(log => (
              <LogEntryItem key={log.id} log={log} />
            ))}
          </Box>
        )}
      </Box>
    </Box>
  );
}
