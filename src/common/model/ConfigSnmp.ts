export interface ConfigSnmp {
    id?: string;
    device_id?: string;
    enabled: boolean;
    version: number;
    community: string;
    port: number;
    timeout_seconds: number;
    retries: number;
}
