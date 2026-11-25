export interface ConfigTermBase {
    port: number;
    term: string;
    username?: string | null;
    password?: string | null;
    keepalive_interval_seconds: number;
    connect_timeout_seconds: number | null;
}


export function createEmptyConfigTermBase(): ConfigTermBase {
    return {
        port: 22,
        username: null,
        password: null,
        term: 'xterm-256color',
        keepalive_interval_seconds: 0,
        connect_timeout_seconds: null,
    };
}