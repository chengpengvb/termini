export interface RespAiSshBatchItem {
    ok: boolean;
    name: string;
    host: string;
    output: string;
    device_id: string;
    error: string | null;
    exit_code: number | null;
}
