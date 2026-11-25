import {Typography} from "@mui/material";
import {useEffect, useState} from "react";
import {PROTOCOL} from "@/common/const/Const.ts";
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNotEmpty} from "@/common/util/StringUtil.ts";
import type {ReqDevice} from "@/common/model/req/ReqDevice.ts";
import {useDeviceService} from "@/common/context/useAppHooks.tsx";

export interface AuthLoginUsernameProps {
    deviceId: string;
    protocol?: number;
}

export function AuthLoginUsername(props: AuthLoginUsernameProps) {
    const {t} = useI18n();
    const {deviceId} = props;
    const protocol = props.protocol ?? PROTOCOL.SSH;
    const deviceService = useDeviceService();
    const [loginUsername, setLoginUsername] = useState<string | null>(null);

    const resolveLoginUsernameFromDevice = (loadedDevice: ReqDevice | null): string | null => {
        if (!loadedDevice) {
            return null;
        }
        const base = protocol === PROTOCOL.RDP ? loadedDevice.config_rdp?.base : loadedDevice.config_ssh?.base;
        if (!base) {
            return null;
        }
        const username = base.username;
        if (!isNotEmpty(username)) {
            return null;
        }
        return username;
    };

    useEffect(() => {
        if (!isNotEmpty(deviceId)) {
            setLoginUsername(null);
            return;
        }
        let cancelled = false;
        deviceService.selectWithConfig(deviceId).then((loadedDevice) => {
            if (cancelled) {
                return;
            }
            setLoginUsername(resolveLoginUsernameFromDevice(loadedDevice));
        }).catch(() => {
            if (cancelled) {
                return;
            }
            setLoginUsername(null);
        });
        return () => {
            cancelled = true;
        };
    }, [deviceId, deviceService, protocol]);

    if (!loginUsername) {
        return null;
    }

    return (
        <Typography sx={{fontSize: 12, color: "var(--tm-text-secondary)", lineHeight: 1.6}}>
            {t('protocol.auth.currentLogin')}
            <Typography component="span" sx={{fontSize: 12, fontWeight: 600, color: "var(--tm-text-primary)"}}>
                {loginUsername}
            </Typography>
        </Typography>
    );
}
