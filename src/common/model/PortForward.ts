import {isEmpty} from "../util/StringUtil";

export const FORWARD_KIND = {
    LOCAL: 0,
    REMOTE: 1,
    DYNAMIC: 2,
} as const;

export const FORWARD_STATUS = {
    IDLE: 0,
    RUNNING: 1,
    ERROR: 2,
} as const;

export interface PortForward {
    id?: string;
    name?: string;
    kind: number;
    status?: number;
    enabled: boolean;
    bind_port: number;
    device_id?: string;
    target_port: number;
    bind_address?: string;
    target_host?: string;
}

export function createEmptyPortForward(): PortForward {
    return {
        bind_port: 0,
        target_port: 0,
        enabled: false,
        kind: FORWARD_KIND.LOCAL,
        status: FORWARD_STATUS.IDLE,
    };
}

function isValidPort(port: number): boolean {
    return Number.isInteger(port) && port > 0 && port <= 65535;
}

export function verifyPortForward(value: PortForward): boolean {
    if (isEmpty(value.name) || value.name.length > 64) {
        return false;
    }
    if (isEmpty(value.device_id)) {
        return false;
    }
    if (isEmpty(value.bind_address) || !isValidPort(value.bind_port)) {
        return false;
    }
    if (value.kind === FORWARD_KIND.DYNAMIC) {
        return true;
    }
    return !isEmpty(value.target_host) && isValidPort(value.target_port);
}
