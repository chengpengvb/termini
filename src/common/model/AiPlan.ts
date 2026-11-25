export const PLAN_SCHEDULE = {
    CYCLE: 1,
    INTERVAL: 2,
    ONCE: 3,
} as const;

export const PLAN_CYCLE_EVERY_DAY = 0;

export const PLAN_RUN_STATUS = {
    RUNNING: 1,
    SUCCESS: 2,
    FAILED: 3,
} as const;

export interface AiPlanTask {
    id: string;
    name: string;
    enabled: boolean;
    prompt: string;
    workspace_dir?: string | null;
    ai_provide_id: string;
    schedule_kind: number;
    cycle_day: number;
    time_minute: number;
    interval_hours: number;
    weekdays: string;
    once_at: number;
    last_run_at: number;
    updated_at: number;
}

export interface AiPlanRun {
    id: string;
    task_id: string;
    chat_id: string;
    status: number;
    started_at: number;
    ended_at: number;
    summary?: string | null;
    error?: string | null;
    task_name: string;
}

export function createEmptyPlanTask(): AiPlanTask {
    return {
        id: '',
        name: '',
        enabled: true,
        prompt: '',
        workspace_dir: null,
        ai_provide_id: '',
        schedule_kind: PLAN_SCHEDULE.CYCLE,
        cycle_day: PLAN_CYCLE_EVERY_DAY,
        time_minute: 9 * 60,
        interval_hours: 24,
        weekdays: '',
        once_at: 0,
        last_run_at: 0,
        updated_at: 0,
    };
}
