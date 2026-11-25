import {Computer} from "@mui/icons-material";
import {Box, Typography} from "@mui/material";
import type {ReqDevice} from "@/common/model/req/ReqDevice.ts";
import {resolveProtocolLabel} from "@/page/common/util/protocolOpenUtil.ts";

export interface HeadProps {
    protocol?: number;
    reqDevice: ReqDevice | null;
}

export function DeviceHead(props: HeadProps) {
    const {reqDevice, protocol} = props;
    if (!reqDevice) {
        return null;
    }
    const deviceName = reqDevice.device.name;
    const deviceHost = reqDevice.device.host;
    const protocolLabel = protocol === undefined ? null : resolveProtocolLabel(protocol);

    return (
        <Box className="tm-protocol-select-head">
            <Box className="tm-protocol-select-device">
                <Box className="tm-protocol-icon-tile tm-protocol-icon-tile--lg is-active">
                    <Computer sx={{fontSize: 18}}/>
                </Box>
                <Box className="tm-protocol-device-text">
                    <Box className="tm-protocol-device-name-row">
                        <Typography className="tm-protocol-device-name">{deviceName}</Typography>
                        {protocolLabel ? <Typography className="tm-protocol-device-protocol">{protocolLabel}</Typography> : null}
                    </Box>
                    <Typography className="tm-protocol-device-ip">{deviceHost}</Typography>
                </Box>
            </Box>
        </Box>
    );
}