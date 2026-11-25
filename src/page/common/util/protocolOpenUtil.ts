import {PROTOCOL} from '@/common/const/Const.ts';
import {createElement, type ReactNode} from 'react';
import {isNotEmpty} from '@/common/util/StringUtil.ts';
import {RdpPage} from '@/page/protocol/rdp/RdpPage.tsx';
import {SshPage} from '@/page/protocol/ssh/SshPage.tsx';
import {SftpPage} from '@/page/protocol/sftp/SftpPage.tsx';
import type {ReqDevice} from '@/common/model/req/ReqDevice.ts';
import type {TabManagerService} from '@/common/components/tab';
import {TelnetPage} from '@/page/protocol/telnet/TelnetPage.tsx';
import {resolveDeviceTabIcon} from '@/page/common/util/deviceUtil.ts';

export function resolveProtocolLabel(protocol: number): string {
    if (protocol === PROTOCOL.SSH) {
        return 'SSH';
    }
    if (protocol === PROTOCOL.TELNET) {
        return 'Telnet';
    }
    if (protocol === PROTOCOL.SFTP) {
        return 'SFTP';
    }
    if (protocol === PROTOCOL.RDP) {
        return 'RDP';
    }
    if (protocol === PROTOCOL.FTP) {
        return 'FTP';
    }
    return '';
}

export function resolveSelectableProtocols(reqDevice: ReqDevice): number[] {
    if (isNotEmpty(reqDevice.device.host)) {
        return [PROTOCOL.SSH, PROTOCOL.SFTP, PROTOCOL.FTP, PROTOCOL.TELNET, PROTOCOL.RDP];
    }
    return [];
}

export async function openDeviceProtocol(params: {
    protocol: number;
    reqDevice: ReqDevice;
    tabManagerService: TabManagerService;
    resolveLocalDevice?: () => Promise<ReqDevice | null>;
}): Promise<boolean> {
    const {protocol, reqDevice, tabManagerService, resolveLocalDevice} = params;
    let content: ReactNode;
    if (protocol === PROTOCOL.SSH) {
        content = createElement(SshPage, {reqDevice});
    } else if (protocol === PROTOCOL.TELNET) {
        content = createElement(TelnetPage, {reqDevice});
    } else if (protocol === PROTOCOL.RDP) {
        content = createElement(RdpPage, {reqDevice});
    } else if (protocol === PROTOCOL.SFTP || protocol === PROTOCOL.FTP) {
        if (!resolveLocalDevice) {
            return false;
        }
        const leftReqDevice = await resolveLocalDevice();
        if (!leftReqDevice) {
            return false;
        }
        content = createElement(SftpPage, {leftReqDevice, rightReqDevice: reqDevice, protocol});
    } else {
        return false;
    }
    tabManagerService.openTab({
        panes: [{
            tabBar: {
                canClose: true,
                title: reqDevice.device.name,
                icon: resolveDeviceTabIcon(reqDevice.device.os_type, protocol),
            },
            content,
        }],
    });
    return true;
}
