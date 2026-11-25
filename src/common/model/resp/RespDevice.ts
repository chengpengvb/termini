import type {Device} from "../Device";
import type {ConfigFtp} from "../ConfigFtp";
import type {ConfigRdp} from "../ConfigRdp";
import type {ConfigSsh} from "../ConfigSsh";
import type {ConfigSnmp} from "../ConfigSnmp";
import type {ConfigTelnet} from "../ConfigTelnet";

export interface RespDevice {
    device: Device;
    group_id?: string | null;
    config_ssh?: ConfigSsh | null;
    config_rdp?: ConfigRdp | null;
    config_ftp?: ConfigFtp | null;
    config_telnet?: ConfigTelnet | null;
    config_snmp?: ConfigSnmp | null;
}
