import type {ReactNode} from 'react';
import {useEffect, useState} from 'react';
import {open} from '@tauri-apps/plugin-dialog';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isEmpty} from '@/common/util/StringUtil.ts';
import {usePaneContext} from '@/common/components/tab';
import {resolveErrorText} from '@/common/model/resp/ResultModel.ts';
import ScheduleOutlined from '@mui/icons-material/ScheduleOutlined';
import {usePlatformMobile} from '@/common/context/usePlatformMobile';
import {EditField} from '@/page/common/components/input/EditField.tsx';
import {PlanPromptEditor} from '@/page/agent/plan/PlanPromptEditor.tsx';
import {ModelSelect} from '@/page/common/components/model/ModelSelect.tsx';
import {SelectField} from '@/page/common/components/input/SelectField.tsx';
import {SwitchField} from '@/page/common/components/input/SwitchField.tsx';
import {Box, Button, ButtonBase, CircularProgress, Typography} from '@mui/material';
import {AlertDialog, type AlertColor} from '@/common/components/dialog/AlertDialog.tsx';
import {useAiPlanService, useTabManagerService} from '@/common/context/useAppHooks.tsx';
import {createEmptyPlanTask, PLAN_CYCLE_EVERY_DAY, PLAN_SCHEDULE, type AiPlanTask} from '@/common/model/AiPlan.ts';

const WEEKDAYS = [1, 2, 3, 4, 5, 6, 7];

const HOUR_OPTIONS = Array.from({length: 24}, (_, index) => ({value: String(index), label: String(index).padStart(2, '0')}));
const MINUTE_OPTIONS = Array.from({length: 12}, (_, index) => ({value: String(index * 5), label: String(index * 5).padStart(2, '0')}));

interface PlanEditPageProps {
    task: AiPlanTask | null;
    onSaved: () => void;
}

function parseWeekdays(raw: string): number[] {
    if (raw.trim().length === 0) {
        return [];
    }
    return raw.split(',').map(item => Number(item.trim())).filter(item => item >= 1 && item <= 7);
}

function toLocalDateInput(millis: number): string {
    const date = millis > 0 ? new Date(millis) : new Date();
    const pad = (value: number): string => String(value).padStart(2, '0');
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

export function PlanEditPage(props: PlanEditPageProps) {
    const {t} = useI18n();
    const {task, onSaved} = props;
    const {paneId} = usePaneContext();
    const mobileViewport = usePlatformMobile();
    const [saving, setSaving] = useState(false);
    const aiPlanService = useAiPlanService();
    const tabManagerService = useTabManagerService();
    const [alert, setAlert] = useState<{type: AlertColor; message: string} | null>(null);
    const [form, setForm] = useState<AiPlanTask>(() => {
        if (task) {
            return {...task};
        }
        return createEmptyPlanTask();
    });
    const selectedWeekdays = parseWeekdays(form.weekdays);
    const [intervalText, setIntervalText] = useState<string>(() => (form.interval_hours > 0 ? String(form.interval_hours) : ''));

    const cycleOptions = [
        {value: String(PLAN_CYCLE_EVERY_DAY), label: t('agent.plan.everyDay')},
        ...WEEKDAYS.map(day => ({value: String(day), label: t(`agent.plan.week${day}`)})),
    ];

    function resolveTitle(): string {
        if (isEmpty(form.id)) {
            return t('agent.plan.addTitle');
        }
        return t('agent.plan.editTitle');
    }

    function canSave(): boolean {
        if (saving) {
            return false;
        }
        if (isEmpty(form.name.trim())) {
            return false;
        }
        if (isEmpty(form.prompt.trim())) {
            return false;
        }
        if (isEmpty(form.ai_provide_id)) {
            return false;
        }
        if (form.schedule_kind === PLAN_SCHEDULE.ONCE) {
            return form.once_at > 0;
        }
        if (form.schedule_kind === PLAN_SCHEDULE.INTERVAL) {
            return form.interval_hours >= 1;
        }
        return true;
    }

    function renderSaveStartIcon(): ReactNode {
        if (!saving) {
            return null;
        }
        return <CircularProgress size={14} color="inherit"/>;
    }

    function handleClose(): void {
        tabManagerService.closePane(paneId);
    }

    function handlePickWorkspace(): void {
        void open({directory: true, multiple: false}).then(selected => {
            if (selected !== null) {
                setForm(prev => ({...prev, workspace_dir: selected}));
            }
        });
    }

    function handleCycleHourChange(raw: string): void {
        setForm(prev => ({...prev, time_minute: Number(raw) * 60 + prev.time_minute % 60}));
    }

    function handleCycleMinuteChange(raw: string): void {
        setForm(prev => ({...prev, time_minute: Math.floor(prev.time_minute / 60) * 60 + Number(raw)}));
    }

    function resolveOnceDate(): Date {
        return form.once_at > 0 ? new Date(form.once_at) : new Date();
    }

    function handleOnceDateChange(raw: string): void {
        const [year, month, day] = raw.split('-').map(Number);
        if (Number.isNaN(year) || Number.isNaN(month) || Number.isNaN(day)) {
            return;
        }
        setForm(prev => {
            const base = prev.once_at > 0 ? new Date(prev.once_at) : new Date();
            base.setFullYear(year, month - 1, day);
            return {...prev, once_at: base.getTime()};
        });
    }

    function handleOnceHourChange(raw: string): void {
        setForm(prev => {
            const base = prev.once_at > 0 ? new Date(prev.once_at) : new Date();
            base.setHours(Number(raw));
            return {...prev, once_at: base.getTime()};
        });
    }

    function handleOnceMinuteChange(raw: string): void {
        setForm(prev => {
            const base = prev.once_at > 0 ? new Date(prev.once_at) : new Date();
            base.setMinutes(Number(raw), 0, 0);
            return {...prev, once_at: base.getTime()};
        });
    }

    function handleIntervalChange(raw: string): void {
        setIntervalText(raw);
        setForm(prev => ({...prev, interval_hours: raw.trim().length === 0 ? 0 : Number(raw)}));
    }

    function handleToggleWeekday(day: number): void {
        const selected = parseWeekdays(form.weekdays);
        const next = selected.includes(day) ? selected.filter(item => item !== day) : [...selected, day].sort((a, b) => a - b);
        setForm(prev => ({...prev, weekdays: next.join(',')}));
    }

    function handleSave(): void {
        if (!canSave()) {
            return;
        }
        setSaving(true);
        aiPlanService.save(form).then(saved => {
            setSaving(false);
            setForm(saved);
            onSaved();
            setAlert({type: 'success', message: t('agent.plan.saveSuccess')});
        }).catch(err => {
            setSaving(false);
            setAlert({type: 'error', message: resolveErrorText(err)});
        });
    }

    useEffect(() => {
        const title = isEmpty(form.name.trim()) ? resolveTitle() : form.name.trim();
        tabManagerService.updatePaneBar(paneId, {
            title,
            canClose: true,
            icon: <ScheduleOutlined sx={{fontSize: 16}}/>,
        });
    }, [form.id, form.name, paneId, tabManagerService]);

    return (
        <Box className="tm-agent-skill-edit">
            <Box className="tm-agent-skill-edit-topbar tm-page-column">
                <Typography component="h1" className="tm-agent-skill-edit-title">{resolveTitle()}</Typography>
                <Box className="tm-agent-skill-edit-actions">
                    <Button disableRipple onClick={handleClose} className="tm-btn tm-btn-secondary">{t('common.close')}</Button>
                    <Button disableRipple disabled={!canSave()} onClick={handleSave} startIcon={renderSaveStartIcon()} className="tm-btn tm-btn-primary">
                        {t('common.save')}
                    </Button>
                </Box>
            </Box>
            <Box className="tm-agent-skill-edit-body">
                <Box className="tm-agent-page tm-page-column tm-agent-skill-edit-stage">
                    <Box className="tm-agent-section">
                        <Box className="tm-agent-section-head">
                            <Typography className="tm-agent-section-title">{t('agent.plan.basicInfo')}</Typography>
                        </Box>
                        <Box className="tm-agent-skill-edit-fields">
                            <EditField label={t('agent.plan.name')} value={form.name} onChange={name => setForm(prev => ({...prev, name}))} placeholder={t('agent.plan.namePlaceholder')} autoFocus/>
                            {mobileViewport ? null : (
                                <Box className="tm-agent-plan-workspace">
                                    <EditField label={t('agent.plan.workspace')} value={form.workspace_dir ?? ''} onChange={dir => setForm(prev => ({...prev, workspace_dir: dir}))} placeholder={t('agent.plan.workspacePlaceholder')}/>
                                    <ButtonBase disableRipple className="tm-btn tm-btn-secondary" onClick={handlePickWorkspace}>{t('agent.plan.pickDir')}</ButtonBase>
                                </Box>
                            )}
                            <ModelSelect variant="row" label={t('agent.plan.model')} selectedId={form.ai_provide_id} onSelect={provide => setForm(prev => ({...prev, ai_provide_id: provide.id ?? ''}))}/>
                        </Box>
                    </Box>
                    <Box className="tm-agent-section">
                        <Box className="tm-agent-section-head">
                            <Typography className="tm-agent-section-title">{t('agent.plan.prompt')}</Typography>
                        </Box>
                        <Box className="tm-agent-skill-edit-fields">
                            <PlanPromptEditor value={form.prompt} onChange={prompt => setForm(prev => ({...prev, prompt}))}/>
                            <Typography className="tm-agent-plan-silent-hint">{t('agent.plan.silentHint')}</Typography>
                        </Box>
                    </Box>
                    <Box className="tm-agent-section">
                        <Box className="tm-agent-section-head">
                            <Typography className="tm-agent-section-title">{t('agent.plan.schedule')}</Typography>
                        </Box>
                        <Box className="tm-agent-skill-edit-fields">
                            <Box className="tm-agent-plan-kind-tabs">
                                {[PLAN_SCHEDULE.CYCLE, PLAN_SCHEDULE.INTERVAL, PLAN_SCHEDULE.ONCE].map(kind => (
                                    <ButtonBase key={kind} disableRipple className={form.schedule_kind === kind ? 'tm-agent-plan-kind-tab is-active' : 'tm-agent-plan-kind-tab'} onClick={() => setForm(prev => ({...prev, schedule_kind: kind}))}>
                                        {kind === PLAN_SCHEDULE.CYCLE ? t('agent.plan.kindCycle') : kind === PLAN_SCHEDULE.INTERVAL ? t('agent.plan.kindInterval') : t('agent.plan.kindOnce')}
                                    </ButtonBase>
                                ))}
                            </Box>
                            {form.schedule_kind === PLAN_SCHEDULE.CYCLE ? (
                                <Box className="tm-agent-plan-schedule-row">
                                    <Box className="tm-agent-plan-cycle-select">
                                        <SelectField value={String(form.cycle_day)} onChange={raw => setForm(prev => ({...prev, cycle_day: Number(raw)}))} options={cycleOptions}/>
                                    </Box>
                                    <Box className="tm-agent-plan-time-select">
                                        <SelectField value={String(Math.floor(form.time_minute / 60))} onChange={handleCycleHourChange} options={HOUR_OPTIONS}/>
                                    </Box>
                                    <Typography className="tm-agent-plan-schedule-text">:</Typography>
                                    <Box className="tm-agent-plan-time-select">
                                        <SelectField value={String(form.time_minute % 60)} onChange={handleCycleMinuteChange} options={MINUTE_OPTIONS}/>
                                    </Box>
                                </Box>
                            ) : null}
                            {form.schedule_kind === PLAN_SCHEDULE.INTERVAL ? (
                                <>
                                    <Box className="tm-agent-plan-schedule-row">
                                        <Typography className="tm-agent-plan-schedule-text">{t('agent.plan.every')}</Typography>
                                        <input type="number" min={1} max={720} className="tm-agent-plan-number-input" value={intervalText} onChange={event => handleIntervalChange(event.target.value)}/>
                                        <Typography className="tm-agent-plan-schedule-text">{t('agent.plan.hours')}</Typography>
                                    </Box>
                                    <Box className="tm-agent-plan-weekdays">
                                        {WEEKDAYS.map(day => (
                                            <ButtonBase key={day} disableRipple className={selectedWeekdays.length === 0 || selectedWeekdays.includes(day) ? 'tm-agent-plan-weekday is-active' : 'tm-agent-plan-weekday'} onClick={() => handleToggleWeekday(day)}>
                                                {t(`agent.plan.week${day}`)}
                                            </ButtonBase>
                                        ))}
                                    </Box>
                                </>
                            ) : null}
                            {form.schedule_kind === PLAN_SCHEDULE.ONCE ? (
                                <Box className="tm-agent-plan-schedule-row">
                                    <input type="date" className="tm-agent-plan-time-input" value={toLocalDateInput(form.once_at)} onChange={event => handleOnceDateChange(event.target.value)}/>
                                    <Box className="tm-agent-plan-time-select">
                                        <SelectField value={String(resolveOnceDate().getHours())} onChange={handleOnceHourChange} options={HOUR_OPTIONS}/>
                                    </Box>
                                    <Typography className="tm-agent-plan-schedule-text">:</Typography>
                                    <Box className="tm-agent-plan-time-select">
                                        <SelectField value={String(resolveOnceDate().getMinutes())} onChange={handleOnceMinuteChange} options={MINUTE_OPTIONS}/>
                                    </Box>
                                </Box>
                            ) : null}
                            <SwitchField label={t('agent.plan.enabled')} value={form.enabled} onChange={enabled => setForm(prev => ({...prev, enabled}))} description={t('agent.plan.enabledDesc')}/>
                        </Box>
                    </Box>
                </Box>
            </Box>
            <AlertDialog duration={1500} open={alert !== null} handleClose={() => setAlert(null)} message={alert === null ? null : alert.message} type={alert === null ? 'error' : alert.type}/>
        </Box>
    );
}
