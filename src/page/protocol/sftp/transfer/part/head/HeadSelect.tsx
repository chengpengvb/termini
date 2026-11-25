import {useEffect, useState} from "react";
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNotNil} from "@/common/util/StringUtil.ts";
import {ReqDevice} from "@/common/model/req/ReqDevice.ts";
import {ArrowBack, Home, Search} from "@mui/icons-material";
import {ConfigTermBase} from "@/common/model/ConfigTermBase.ts";
import {DEVICE_CONNECTION_TYPE} from "@/common/model/Device.ts";
import {useDeviceService} from "@/common/context/useAppHooks.tsx";
import {Box, ButtonBase, InputBase, Typography} from "@mui/material";
import {resolveDeviceOsIcon} from "@/page/common/util/deviceUtil.ts";
import {useBackDismiss} from "@/page/common/hooks/useBackDismiss.ts";

export interface HeadSelectProps {
    onClose: () => void;
    excludeDeviceId: string;
    onConnect: (deviceId: string) => void;
}

export function HeadSelect(props: HeadSelectProps) {
    const {t} = useI18n();
    const deviceService = useDeviceService();
    const {excludeDeviceId, onClose, onConnect} = props;
    const [devices, setDevices] = useState<ReqDevice[]>([]);
    const [searchQuery, setSearchQuery] = useState<string | null>(null);
    const [localDevice, setLocalDevice] = useState<ReqDevice | null>(null);
    const [selectedDeviceId, setSelectedDeviceId] = useState<string | null>(null);


    function resolveProtocolLabels(reqDevice: ReqDevice): string[] {
        const labels: string[] = [];
        if (reqDevice.config_ssh) {
            labels.push("ssh");
        }
        if (reqDevice.config_telnet) {
            labels.push("telnet");
        }
        return labels;
    }

    function resolveBaseUsername(base: ConfigTermBase | undefined): string | null {
        if (!base || !base.username) {
            return null;
        }
        return base.username;
    }

    function resolveUsername(reqDevice: ReqDevice): string | null {
        if (reqDevice.config_ssh) {
            const sshUsername = resolveBaseUsername(reqDevice.config_ssh.base);
            if (sshUsername) {
                return sshUsername;
            }
        }
        if (reqDevice.config_telnet) {
            const telnetUsername = resolveBaseUsername(reqDevice.config_telnet.base);
            if (telnetUsername) {
                return telnetUsername;
            }
        }
        return null;
    }

    function resolveDeviceSubtitle(reqDevice: ReqDevice): string {
        const labels = resolveProtocolLabels(reqDevice);
        const username = resolveUsername(reqDevice);
        if (username) {
            labels.push(username);
        }
        if (labels.length === 0) {
            return reqDevice.device.host;
        }
        return labels.join(", ");
    }

    function resolveDeviceClassName(deviceId: string): string {
        if (selectedDeviceId === deviceId) {
            return "tm-sftp-head-select-item tm-sftp-head-select-item-selected";
        }
        return "tm-sftp-head-select-item";
    }

    function matchesSearch(reqDevice: ReqDevice): boolean {
        if (!searchQuery) {
            return true;
        }
        const query = searchQuery.trim().toLowerCase();
        if (!query) {
            return true;
        }
        const name = reqDevice.device.name.toLowerCase();
        const host = reqDevice.device.host.toLowerCase();
        if (name.includes(query)) {
            return true;
        }
        return host.includes(query);
    }

    function resolveVisibleDevices(): ReqDevice[] {
        const result: ReqDevice[] = [];
        for (const reqDevice of devices) {
            if (reqDevice.device.id === excludeDeviceId) {
                continue;
            }
            if (reqDevice.device.type === DEVICE_CONNECTION_TYPE.LOCAL) {
                continue;
            }
            if (!matchesSearch(reqDevice)) {
                continue;
            }
            result.push(reqDevice);
        }
        return result;
    }

    function handleSearchChange(value: string): void {
        if (!value) {
            setSearchQuery(null);
            return;
        }
        setSearchQuery(value);
    }

    function handleConnectDevice(deviceId: string): void {
        setSelectedDeviceId(deviceId);
        onConnect(deviceId);
    }

    function handleConnectLocal(): void {
        if (!localDevice) {
            return;
        }
        onConnect(localDevice.device.id);
    }

    useBackDismiss(true, onClose, 'sftp-head-select');

    useEffect(() => {
        deviceService.selectAllWithConfig().then();
        deviceService.localDevice().then(setLocalDevice);
        const off = deviceService.addDeviceListChangeListener(setDevices);
        return () => {
            off();
        };
    }, [deviceService]);

    const visibleDevices = resolveVisibleDevices();

    return (
        <Box className="tm-sftp-head-select">
            <Box className="tm-sftp-head-select-header">
                <Box className="tm-sftp-head-select-title-row">
                    <ButtonBase className="tm-sftp-head-select-title-group" onClick={onClose} aria-label={t('common.back')}>
                        <ArrowBack className="tm-sftp-head-select-back-icon"/>
                        <Typography className="tm-sftp-head-select-title">{t('protocol.sftp.selectHost')}</Typography>
                    </ButtonBase>
                    {localDevice ? (
                        <ButtonBase className="tm-sftp-head-select-local" onClick={handleConnectLocal}>
                            <Home className="tm-sftp-head-select-local-icon"/>
                            <Box component="span">{t('common.local')}</Box>
                        </ButtonBase>
                    ) : null}
                </Box>
                <Box className="tm-sftp-head-select-search">
                    <Search className="tm-sftp-head-select-search-icon"/>
                    <InputBase className="tm-sftp-head-select-search-input" placeholder={t('common.search')} value={isNotNil(searchQuery) ? searchQuery : String()} onChange={(e) => handleSearchChange(e.target.value)} />
                </Box>
            </Box>
            <Box className="tm-sftp-head-select-body">
                <Typography className="tm-sftp-head-select-section-title">{t('common.hosts')}</Typography>
                <Box className="tm-sftp-head-select-list">
                    {visibleDevices.map((reqDevice) => {
                        const device = reqDevice.device;
                        return (
                            <ButtonBase key={device.id} className={resolveDeviceClassName(device.id)} onClick={() => handleConnectDevice(device.id)}>
                                <Box className="tm-sftp-head-select-item-icon">
                                    {resolveDeviceOsIcon(device.os_type, "tm-sftp-head-select-item-icon-svg")}
                                </Box>
                                <Box className="tm-sftp-head-select-item-info">
                                    <Typography className="tm-sftp-head-select-item-name tm-ellipsis">{device.name}</Typography>
                                    <Typography className="tm-sftp-head-select-item-subtitle tm-ellipsis">{resolveDeviceSubtitle(reqDevice)}</Typography>
                                </Box>
                            </ButtonBase>
                        );
                    })}
                </Box>
            </Box>
        </Box>
    );
}
