import { useI18n } from '@/common/i18n/useI18n.ts';
import { isEmpty } from '@/common/util/StringUtil.ts';
import { formatDate } from '@/common/util/DateUtil.ts';
import { createElement, useEffect, useState } from 'react';
import { PlanEditPage } from '@/page/agent/plan/PlanEditPage.tsx';
import { useKeepAwake } from '@/page/common/hooks/useKeepAwake.ts';
import { Box, ButtonBase, Switch, Typography } from '@mui/material';
import ScheduleOutlined from '@mui/icons-material/ScheduleOutlined';
import { resolveErrorText } from '@/common/model/resp/ResultModel.ts';
import { AlertDialog } from '@/common/components/dialog/AlertDialog.tsx';
import { Add, DeleteOutline, Edit, PlayArrow } from '@mui/icons-material';
import { useServiceContainer } from '@/common/context/ServiceContext.tsx';
import { ConfirmDialog } from '@/common/components/dialog/ConfirmDialog.tsx';
import { WebSocketServiceImpl } from '@/service/impl/WebSocketServiceImpl.ts';
import { RESP_EVENT_AI_PLAN } from '@/common/model/resp/event/RespEventAiPlan.ts';
import { useAiPlanService, useTabManagerService } from '@/common/context/useAppHooks.tsx';
import { usePlatformMobile, usePlatformNativeMobile } from '@/common/context/usePlatformMobile';
import { PLAN_CYCLE_EVERY_DAY, PLAN_RUN_STATUS, PLAN_SCHEDULE, type AiPlanRun, type AiPlanTask } from '@/common/model/AiPlan.ts';

const TAB = { TASKS: 1, RUNS: 2 } as const;

function formatMinute(timeMinute: number): string {
  const hour = Math.floor(timeMinute / 60);
  const minute = timeMinute % 60;
  return `${String(hour).padStart(2, '0')}:${String(minute).padStart(2, '0')}`;
}

export function PlanManage(props: { onOpenChat: (chatId: string) => void }) {
  const { t, tf } = useI18n();
  const { onOpenChat } = props;
  const container = useServiceContainer();
  const aiPlanService = useAiPlanService();
  const mobileViewport = usePlatformMobile();
  const nativeMobile = usePlatformNativeMobile();
  const tabManagerService = useTabManagerService();
  const [tab, setTab] = useState<number>(TAB.TASKS);
  const [runs, setRuns] = useState<AiPlanRun[]>([]);
  const [tasks, setTasks] = useState<AiPlanTask[]>([]);
  const [pendingDelete, setPendingDelete] = useState<AiPlanTask | null>(null);
  const [alertMessage, setAlertMessage] = useState<string | null>(null);

  const reload = (): void => {
    aiPlanService
      .selectAll()
      .then(list => {
        setTasks([...list].sort((a, b) => b.updated_at - a.updated_at));
      })
      .catch(err => setAlertMessage(resolveErrorText(err)));
    aiPlanService
      .runs()
      .then(setRuns)
      .catch(err => setAlertMessage(resolveErrorText(err)));
  };

  const resolveScheduleText = (item: AiPlanTask): string => {
    if (item.schedule_kind === PLAN_SCHEDULE.CYCLE) {
      const day = item.cycle_day === PLAN_CYCLE_EVERY_DAY ? t('agent.plan.everyDay') : t(`agent.plan.week${item.cycle_day}`);
      return `${day} ${formatMinute(item.time_minute)}`;
    }
    if (item.schedule_kind === PLAN_SCHEDULE.INTERVAL) {
      return tf('agent.plan.everyHours', { n: item.interval_hours });
    }
    return tf('agent.plan.onceAt', { date: formatDate(item.once_at) });
  };

  const resolveRunStatus = (run: AiPlanRun): string => {
    if (run.status === PLAN_RUN_STATUS.RUNNING) {
      return t('agent.plan.statusRunning');
    }
    if (run.status === PLAN_RUN_STATUS.SUCCESS) {
      return t('agent.plan.statusSuccess');
    }
    return t('agent.plan.statusFailed');
  };

  const resolveRunStatusClass = (run: AiPlanRun): string => {
    if (run.status === PLAN_RUN_STATUS.RUNNING) {
      return 'tm-agent-plan-status is-running';
    }
    if (run.status === PLAN_RUN_STATUS.SUCCESS) {
      return 'tm-agent-plan-status is-success';
    }
    return 'tm-agent-plan-status is-failed';
  };

  const openPlanEditTab = (task: AiPlanTask | null): void => {
    let title = t('agent.plan.addTitle');
    if (task && !isEmpty(task.name)) {
      title = task.name;
    }
    tabManagerService.openTab({
      panes: [
        {
          tabBar: {
            title,
            canClose: true,
            icon: createElement(ScheduleOutlined, { sx: { fontSize: 16 } }),
          },
          content: createElement(PlanEditPage, { task, onSaved: reload }),
        },
      ],
    });
  };

  const handleToggle = (item: AiPlanTask, enabled: boolean): void => {
    aiPlanService
      .save({ ...item, enabled })
      .then(reload)
      .catch(err => setAlertMessage(resolveErrorText(err)));
  };

  const handleRunNow = (item: AiPlanTask): void => {
    setTab(TAB.RUNS);
    aiPlanService
      .runNow(item.id)
      .then(reload)
      .catch(err => setAlertMessage(resolveErrorText(err)));
  };

  const handleConfirmDelete = (): void => {
    if (pendingDelete === null) {
      return;
    }
    const id = pendingDelete.id;
    setPendingDelete(null);
    aiPlanService
      .deleteWithRuns(id)
      .then(reload)
      .catch(err => setAlertMessage(resolveErrorText(err)));
  };

  const handleDeleteRun = (run: AiPlanRun): void => {
    const action = run.status === PLAN_RUN_STATUS.RUNNING ? aiPlanService.cancelRun(run.id).then(() => aiPlanService.deleteRun(run.id)) : aiPlanService.deleteRun(run.id);
    action.then(reload).catch(err => setAlertMessage(resolveErrorText(err)));
  };

  useKeepAwake(nativeMobile);

  useEffect(() => {
    reload();
    return container.resolve(WebSocketServiceImpl).addWsListener(context => {
      if (context.type === RESP_EVENT_AI_PLAN.ON_CHANGED) {
        reload();
      }
    });
  }, []);

  return (
    <Box className="tm-agent-skill-manage">
      <Box className="tm-agent-page tm-page-column">
        <Box className="tm-agent-section">
          <Box className={mobileViewport ? 'tm-agent-section-head is-mobile-add' : 'tm-agent-section-head'}>
            <Box className="tm-agent-plan-tabs">
              <ButtonBase disableRipple className={tab === TAB.TASKS ? 'tm-agent-plan-tab is-active' : 'tm-agent-plan-tab'} onClick={() => setTab(TAB.TASKS)}>
                {t('agent.plan.tabTasks')}
              </ButtonBase>
              <ButtonBase disableRipple className={tab === TAB.RUNS ? 'tm-agent-plan-tab is-active' : 'tm-agent-plan-tab'} onClick={() => setTab(TAB.RUNS)}>
                {t('agent.plan.tabRuns')}
              </ButtonBase>
            </Box>
            {tab === TAB.TASKS ? (
              <ButtonBase disableRipple className={mobileViewport ? 'tm-mobile-header-action' : 'tm-agent-section-action'} aria-label={t('agent.plan.add')} onClick={() => openPlanEditTab(null)}>
                {mobileViewport ? (
                  <>
                    <Add className="tm-mobile-header-action-icon" />
                    <Box component="span" className="tm-mobile-header-action-label">
                      {t('common.add')}
                    </Box>
                  </>
                ) : (
                  <Add sx={{ fontSize: 18 }} />
                )}
              </ButtonBase>
            ) : null}
          </Box>
          {tab === TAB.TASKS ? (
            tasks.length === 0 ? (
              <Box className="tm-agent-page-empty" role={mobileViewport ? undefined : 'button'} tabIndex={mobileViewport ? undefined : 0} onClick={mobileViewport ? undefined : () => openPlanEditTab(null)}>
                <Typography className="tm-agent-page-empty-title">{t('agent.plan.emptyTasks')}</Typography>
                {mobileViewport ? null : <Typography className="tm-agent-page-empty-message">{t('agent.plan.keepAliveHint')}</Typography>}
                {mobileViewport ? (
                  <ButtonBase disableRipple className="tm-mobile-header-action tm-empty-state-cta" onClick={() => openPlanEditTab(null)}>
                    <Add className="tm-mobile-header-action-icon" />
                    <Box component="span" className="tm-mobile-header-action-label">
                      {t('common.add')}
                    </Box>
                  </ButtonBase>
                ) : null}
              </Box>
            ) : (
              <>
                <Typography className="tm-agent-plan-hint">{t('agent.plan.keepAliveHint')}</Typography>
                <Box className="tm-agent-list">
                  {tasks.map(item => (
                    <Box key={item.id} className="tm-agent-list-row" role="button" tabIndex={0} onClick={() => openPlanEditTab(item)}>
                      <Box className="tm-agent-list-row-icon" aria-hidden>
                        <ScheduleOutlined sx={{ fontSize: 18 }} />
                      </Box>
                      <Box className="tm-agent-list-row-main">
                        <Typography className="tm-agent-list-row-name">{item.name}</Typography>
                        <Typography className="tm-agent-list-row-desc">
                          {resolveScheduleText(item)}
                          {item.last_run_at > 0 ? ` · ${tf('agent.plan.lastRun', { date: formatDate(item.last_run_at) })}` : ''}
                        </Typography>
                      </Box>
                      <Box className="tm-agent-list-row-actions is-visible" onClick={event => event.stopPropagation()}>
                        <Switch size="small" checked={item.enabled} onChange={(_, checked) => handleToggle(item, checked)} />
                        <ButtonBase disableRipple className="tm-agent-list-row-btn" aria-label={t('agent.plan.runNow')} onClick={() => handleRunNow(item)}>
                          <PlayArrow sx={{ fontSize: 16 }} />
                        </ButtonBase>
                        {mobileViewport ? null : (
                          <ButtonBase disableRipple className="tm-agent-list-row-btn" aria-label={t('common.edit')} onClick={() => openPlanEditTab(item)}>
                            <Edit sx={{ fontSize: 15 }} />
                          </ButtonBase>
                        )}
                        <ButtonBase disableRipple className="tm-agent-list-row-btn is-danger" aria-label={t('common.delete')} onClick={() => setPendingDelete(item)}>
                          <DeleteOutline sx={{ fontSize: 16 }} />
                        </ButtonBase>
                      </Box>
                    </Box>
                  ))}
                </Box>
              </>
            )
          ) : runs.length === 0 ? (
            <Box className="tm-agent-page-empty">
              <Typography className="tm-agent-page-empty-title">{t('agent.plan.emptyRuns')}</Typography>
            </Box>
          ) : (
            <Box className="tm-agent-list">
              {runs.map(run => (
                <Box key={run.id} className="tm-agent-list-row" role="button" tabIndex={0} onClick={() => onOpenChat(run.chat_id)}>
                  <Box className="tm-agent-list-row-main">
                    <Box className="tm-agent-plan-run-head">
                      <Typography className="tm-agent-list-row-name">{run.task_name}</Typography>
                      <Typography component="span" className={resolveRunStatusClass(run)}>
                        {resolveRunStatus(run)}
                      </Typography>
                    </Box>
                    <Typography className="tm-agent-list-row-desc">
                      {formatDate(run.started_at)}
                      {run.error ? ` · ${run.error}` : run.summary ? ` · ${run.summary}` : ''}
                    </Typography>
                  </Box>
                  <Box className="tm-agent-list-row-actions is-visible">
                    <ButtonBase
                      disableRipple
                      className="tm-agent-list-row-btn is-danger"
                      aria-label={t('common.delete')}
                      title={t('common.delete')}
                      onClick={event => {
                        event.stopPropagation();
                        handleDeleteRun(run);
                      }}
                    >
                      <DeleteOutline sx={{ fontSize: 16 }} />
                    </ButtonBase>
                  </Box>
                </Box>
              ))}
            </Box>
          )}
        </Box>
      </Box>
      <ConfirmDialog
        open={pendingDelete !== null}
        title={t('agent.plan.deleteTitle')}
        message={pendingDelete === null ? '' : tf('agent.plan.deleteMessage', { name: pendingDelete.name })}
        confirmLabel={t('common.delete')}
        danger
        onCancel={() => setPendingDelete(null)}
        onConfirm={handleConfirmDelete}
      />
      <AlertDialog duration={3000} open={alertMessage !== null} handleClose={() => setAlertMessage(null)} message={alertMessage} type="error" />
    </Box>
  );
}
