import {Box} from "@mui/material";
import {ReqDevice} from "@/common/model/req/ReqDevice.ts";
import {resolveDeviceOsIcon, resolveDeviceOsIconChipStyle, resolveDeviceOsIconColor} from "@/page/common/util/deviceUtil.ts";

export interface HeadIdentityProps {
    reqDevice: ReqDevice;
    onClick: () => void;
}

export function HeadIdentity(props: HeadIdentityProps) {
    const {onClick} = props;
    const {device} = props.reqDevice;
    const hasAccent = !!resolveDeviceOsIconColor(device.os_type);

    return (
        <Box className="tm-sftp-head-identity" onClick={onClick} role="button" tabIndex={0}>
            <Box className={hasAccent ? 'tm-sftp-head-identity-icon has-accent' : 'tm-sftp-head-identity-icon'} style={resolveDeviceOsIconChipStyle(device.os_type)}>
                {resolveDeviceOsIcon(device.os_type, "tm-sftp-head-identity-icon-svg")}
            </Box>
            <Box component="span" className="tm-sftp-head-identity-title">{device.name}</Box>
        </Box>
    );
}
