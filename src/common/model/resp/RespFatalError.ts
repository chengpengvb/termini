export const FATAL_EVENT = 'termini://fatal';

export const FATAL_KIND = {
    PANIC: 1,
    CRYPTO: 2,
} as const;

export interface RespFatalError {
    kind: number;
    message: string;
}
