export interface JobIndicatorValue {
    id: string;
    success: boolean;
    updated_at: number;
    value?: string | null;
    error?: string | null;
    job_device_id: string;
    job_indicator_id: string;
}
