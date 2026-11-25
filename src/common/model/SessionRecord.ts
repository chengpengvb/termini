export interface SessionRecord {
    id: string;
    device_name: string;
    protocol: string;
    file_name: string;
    width: number;
    height: number;
    started_at: number;
    ended_at: number;
    size_bytes: number;
}
