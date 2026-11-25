export interface RespModel {
    name: string;
    max_token: number;
    vision?: boolean;
    no_thinking?: Record<string, unknown> | null;
}

export interface RespProvider {
    name: string;
    provide_id: number;
    models: RespModel[];
    default_url: string | null;
}
