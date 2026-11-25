import {isNil} from "@/common/util/StringUtil.ts";
import {translate, translateFormat} from '@/common/i18n/i18n.ts';
import {FORWARD_KIND, FORWARD_STATUS, PortForward} from "@/common/model/PortForward.ts";

export function resolveForwardTypeTabs(): { value: number; label: string }[] {
    return [
        {value: FORWARD_KIND.LOCAL, label: translate(`home.forward.kind.${FORWARD_KIND.LOCAL}`)},
        {value: FORWARD_KIND.REMOTE, label: translate(`home.forward.kind.${FORWARD_KIND.REMOTE}`)},
        {value: FORWARD_KIND.DYNAMIC, label: translate(`home.forward.kind.${FORWARD_KIND.DYNAMIC}`)},
    ];
}

export function resolveForwardKindLabel(kind: number): string {
    return translate(`home.forward.kind.${kind}`);
}

export function resolveForwardKindHint(kind: number): string {
    return translate(`home.forward.kindHint.${kind}`);
}

export function resolveForwardStatusLabel(status: number | undefined): string {
    if (isNil(status)) {
        return '';
    }
    return translate(`home.forward.status.${status}`);
}

export function resolveForwardStatusColor(status: number | undefined): string {
    if (status === FORWARD_STATUS.RUNNING) {
        return 'var(--tm-status-success)';
    }
    if (status === FORWARD_STATUS.ERROR) {
        return 'var(--tm-status-danger)';
    }
    return 'var(--tm-text-tertiary)';
}

export function resolveForwardSummary(value: PortForward): string {
    const kind = resolveForwardKindLabel(value.kind);
    const bind = `${value.bind_address}:${value.bind_port}`;
    if (value.kind === FORWARD_KIND.DYNAMIC) {
        return translateFormat('home.forward.summaryDynamic', {kind, bind});
    }
    const target = `${value.target_host}:${value.target_port}`;
    if (value.kind === FORWARD_KIND.REMOTE) {
        return translateFormat('home.forward.summaryRemote', {kind, bind, target});
    }
    return translateFormat('home.forward.summaryLocal', {kind, bind, target});
}
