import {Fullscreen} from '@mui/icons-material';
import {translate} from '@/common/i18n/i18n.ts';
import type {TerminalTabAction} from '@/page/protocol/common/terminal/TerminalSettingsAction.tsx';

export function createRdpFullscreenAction(onEnter: () => void): TerminalTabAction {
    return {
        actionTitle: translate('protocol.rdp.fullscreen'),
        actionIcon: <Fullscreen className="tm-rdp-tab-fs" fontSize="inherit"/>,
        onActionClick: onEnter,
    };
}
