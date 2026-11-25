export interface StepEvent {
    stop: boolean,// 是否停止
    error: boolean,// 是否错误
    progress: number,// 进度百分比
    opened: boolean,// 是否打开
    data?: unknown | null,// 额外数据
    stepTypeMessage?: string,// 步骤信息/错误信息
}