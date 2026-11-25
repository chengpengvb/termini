import { useState } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { Box, ButtonBase, Collapse } from '@mui/material';
import { isNil, isNotNil } from '@/common/util/StringUtil.ts';
import { useDeviceService } from '@/common/context/useAppHooks.tsx';
import { AgentShellResult } from '@/page/agent/common/AgentShellResult.tsx';
import { KeyboardArrowDown, KeyboardArrowRight } from '@mui/icons-material';
import type { RespAiSshBatchItem } from '@/common/model/resp/RespAiSshBatch.ts';

interface DeviceBatchViewProps {
  done: boolean;
  deviceIds: string[];
  chunks: ReadonlyMap<string, string>;
  items: RespAiSshBatchItem[] | null;
}

const ROW_STATE = {
  RUNNING: 0,
  OK: 1,
  FAILED: 2,
  CANCELLED: 3,
} as const;

type RowState = (typeof ROW_STATE)[keyof typeof ROW_STATE];

const ROW_STATE_CLASS: Record<RowState, string> = {
  [ROW_STATE.RUNNING]: 'is-running',
  [ROW_STATE.OK]: 'is-ok',
  [ROW_STATE.FAILED]: 'is-failed',
  [ROW_STATE.CANCELLED]: 'is-cancelled',
};

interface DeviceRow {
  deviceId: string;
  state: RowState;
  item: RespAiSshBatchItem | null;
  live: string;
}

function resolveState(item: RespAiSshBatchItem | null, done: boolean): RowState {
  if (isNil(item)) {
    return done ? ROW_STATE.FAILED : ROW_STATE.RUNNING;
  }
  if (item.ok) {
    return ROW_STATE.OK;
  }
  return item.error === 'cancelled' ? ROW_STATE.CANCELLED : ROW_STATE.FAILED;
}

export function DeviceBatchView(props: DeviceBatchViewProps) {
  const { t, tf } = useI18n();
  const deviceService = useDeviceService();
  const { done, deviceIds, chunks, items } = props;
  const [openIds, setOpenIds] = useState<ReadonlySet<string>>(new Set());

  const ordered: string[] = [...deviceIds];
  for (const id of chunks.keys()) {
    if (!ordered.includes(id)) {
      ordered.push(id);
    }
  }
  for (const item of items ?? []) {
    if (!ordered.includes(item.device_id)) {
      ordered.push(item.device_id);
    }
  }
  const rows: DeviceRow[] = ordered.map(deviceId => {
    const item = items?.find(row => row.device_id === deviceId) ?? null;
    return { deviceId, item, live: chunks.get(deviceId) ?? '', state: resolveState(item, done) };
  });

  function toggle(deviceId: string): void {
    setOpenIds(current => {
      const next = new Set(current);
      if (next.has(deviceId)) {
        next.delete(deviceId);
      } else {
        next.add(deviceId);
      }
      return next;
    });
  }

  function resolveLabel(row: DeviceRow): string {
    const registered = deviceService.resolveDeviceLabel(row.deviceId);
    if (isNotNil(registered) && registered.length > 0) {
      return registered;
    }
    if (isNotNil(row.item) && row.item.name.length > 0) {
      return row.item.host.length > 0 ? `${row.item.name} (${row.item.host})` : row.item.name;
    }
    return row.deviceId;
  }

  function resolveMeta(row: DeviceRow): string {
    if (row.state === ROW_STATE.RUNNING) {
      return t('agent.tool.deviceBatchRowRunning');
    }
    if (row.state === ROW_STATE.CANCELLED) {
      return t('agent.tool.deviceBatchRowCancelled');
    }
    if (isNotNil(row.item) && isNotNil(row.item.exit_code)) {
      return `exit_code=${row.item.exit_code}`;
    }
    return row.state === ROW_STATE.OK ? t('agent.tool.deviceBatchRowOk') : t('agent.tool.deviceBatchRowFailed');
  }

  function resolveBody(row: DeviceRow): string {
    if (isNotNil(row.item)) {
      if (isNotNil(row.item.error) && row.item.error.length > 0) {
        return row.item.output.length > 0 ? `${row.item.output}\n${row.item.error}` : row.item.error;
      }
      return row.item.output.length > 0 ? row.item.output : t('agent.tool.deviceBatchNoOutput');
    }
    return row.live;
  }

  const okCount = rows.filter(row => row.state === ROW_STATE.OK).length;
  const failedCount = rows.filter(row => row.state === ROW_STATE.FAILED || row.state === ROW_STATE.CANCELLED).length;
  const runningCount = rows.filter(row => row.state === ROW_STATE.RUNNING).length;
  const summary = done ? tf('agent.tool.deviceBatchDone', { total: rows.length, ok: okCount, failed: failedCount }) : tf('agent.tool.deviceBatchRunning', { total: rows.length, running: runningCount });

  return (
    <Box className="tm-agent-device-batch">
      <Box className="tm-agent-device-batch-summary">{summary}</Box>
      <Box className="tm-agent-tool-batch">
        {rows.map(row => {
          const open = openIds.has(row.deviceId);
          const body = resolveBody(row);
          return (
            <Box key={row.deviceId} className={`tm-agent-tool-card tm-agent-device-batch-row ${ROW_STATE_CLASS[row.state]}`}>
              <Box className="tm-agent-tool-head">
                <ButtonBase disableRipple className="tm-agent-tool-fold" onClick={() => toggle(row.deviceId)}>
                  <Box component="span" className="tm-agent-tool-chevron">
                    {open ? <KeyboardArrowDown sx={{ fontSize: 16 }} /> : <KeyboardArrowRight sx={{ fontSize: 16 }} />}
                  </Box>
                  <Box component="span" className="tm-agent-device-batch-dot" aria-hidden="true" />
                  <Box component="span" className="tm-agent-tool-title" title={row.deviceId}>
                    {resolveLabel(row)}
                  </Box>
                </ButtonBase>
                <Box component="span" className="tm-agent-device-batch-meta">
                  {resolveMeta(row)}
                </Box>
              </Box>
              <Collapse in={open} unmountOnExit timeout={0}>
                <Box className="tm-agent-tool-body">
                  {body.length === 0 ? null : <AgentShellResult content={body} running={row.state === ROW_STATE.RUNNING} active={open} />}
                </Box>
              </Collapse>
            </Box>
          );
        })}
      </Box>
    </Box>
  );
}
