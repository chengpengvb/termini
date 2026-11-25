import type {ReactNode} from 'react';
import {Settings} from '@mui/icons-material';
import {translate} from '@/common/i18n/i18n.ts';

export interface TerminalTabAction {
    actionIcon: ReactNode;
    actionTitle: string;
    onActionClick: () => void;
}

export function createTerminalSettingsAction(onToggle: () => void): TerminalTabAction {
    return {
        actionTitle: translate('protocol.terminal.settings'),
        actionIcon: <Settings fontSize="inherit"/>,
        onActionClick: onToggle,
    };
}
