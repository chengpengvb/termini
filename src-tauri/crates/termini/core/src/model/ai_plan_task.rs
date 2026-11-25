use termini_macros::base_target;
use serde::{Deserialize, Serialize};
use chrono::{Datelike, Local, TimeZone, Timelike};

pub const PLAN_SCHEDULE_CYCLE: i16 = 1; //周期：每天/每周某天 定点执行
pub const PLAN_SCHEDULE_INTERVAL: i16 = 2; //按间隔：每 N 小时，可限定星期
pub const PLAN_SCHEDULE_ONCE: i16 = 3; //单次：指定时间执行一次

pub const PLAN_CYCLE_EVERY_DAY: i16 = 0; //cycle_day: 0=每天，1..7=周一..周日

const CYCLE_FIRE_GRACE_MS: i64 = 5 * 60 * 1000; //周期任务错过定点后的补跑窗口

#[base_target(table = "ai_plan_task", entity_type = 23)]
#[derive(Clone, Serialize, Deserialize)]
pub struct Model {
    pub name: String,
    pub enabled: bool,
    pub prompt: String,                // 任务提示词，设备/技能用 @[device:id] @[skill:id] 引用，与聊天输入一致
    pub workspace_dir: Option<String>, // 工作空间目录（可选），产出物（报告/文档等）写入该目录
    pub ai_provide_id: String,         // 执行用的模型
    pub schedule_kind: i16,            // PLAN_SCHEDULE_*
    pub cycle_day: i16,                // 周期：PLAN_CYCLE_EVERY_DAY 或 ISO 星期 1..7
    pub time_minute: i16,              // 周期：当天第几分钟（HH*60+mm）
    pub interval_hours: i16,           // 按间隔：每 N 小时
    pub weekdays: String,              // 按间隔：勾选的 ISO 星期，如 "1,2,3,4,5"；空 = 每天
    pub once_at: i64,                  // 单次：执行时间（epoch ms）
    pub last_run_at: i64,
    pub updated_at: i64,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            enabled: true,
            prompt: String::new(),
            workspace_dir: None,
            ai_provide_id: String::new(),
            schedule_kind: PLAN_SCHEDULE_CYCLE,
            cycle_day: PLAN_CYCLE_EVERY_DAY,
            time_minute: 9 * 60,
            interval_hours: 24,
            weekdays: String::new(),
            once_at: 0,
            last_run_at: 0,
            updated_at: 0,
        }
    }
}

impl Model {
    pub fn is_due(&self, now_ms: i64) -> bool {
        if !self.enabled {
            return false;
        }
        match self.schedule_kind {
            PLAN_SCHEDULE_CYCLE => self.cycle_due(now_ms),
            PLAN_SCHEDULE_INTERVAL => self.interval_due(now_ms),
            PLAN_SCHEDULE_ONCE => self.once_due(now_ms),
            _ => false,
        }
    }

    fn cycle_due(&self, now_ms: i64) -> bool {
        let now = Local.timestamp_millis_opt(now_ms).single().expect("invalid now_ms");
        let weekday = now.weekday().number_from_monday() as i16;
        if self.cycle_day != PLAN_CYCLE_EVERY_DAY && self.cycle_day != weekday {
            return false;
        }
        let minute_of_day = (now.hour() * 60 + now.minute()) as i16;
        if minute_of_day < self.time_minute {
            return false;
        }
        let scheduled_ms = now_ms - i64::from(minute_of_day - self.time_minute) * 60 * 1000 - i64::from(now.second()) * 1000;
        if now_ms - scheduled_ms > CYCLE_FIRE_GRACE_MS {
            return false;
        }
        self.last_run_at < scheduled_ms
    }

    fn interval_due(&self, now_ms: i64) -> bool {
        if self.interval_hours <= 0 {
            return false;
        }
        let now = Local.timestamp_millis_opt(now_ms).single().expect("invalid now_ms");
        let weekday = now.weekday().number_from_monday() as i16;
        if !self.weekday_selected(weekday) {
            return false;
        }
        now_ms - self.last_run_at >= i64::from(self.interval_hours) * 3600 * 1000
    }

    fn once_due(&self, now_ms: i64) -> bool {
        if self.once_at <= 0 {
            return false;
        }
        now_ms >= self.once_at && self.last_run_at < self.once_at
    }

    fn weekday_selected(&self, weekday: i16) -> bool {
        if self.weekdays.trim().is_empty() {
            return true;
        }
        self.weekdays.split(',').filter_map(|item| item.trim().parse::<i16>().ok()).any(|item| item == weekday)
    }
}

pub type AiPlanTask = Model;
