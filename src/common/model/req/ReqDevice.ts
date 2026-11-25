import type {Device} from "../Device";
import {RDP_SECURITY} from "../ConfigRdp";
import type {ConfigRdp} from "../ConfigRdp";
import type {ConfigSsh} from "../ConfigSsh";
import type {ConfigSnmp} from "../ConfigSnmp";
import type {ConfigTelnet} from "../ConfigTelnet";
import {DEFAULT_GROUP_ID} from "@/common/model/Group";
import {FTP_SECURITY, type ConfigFtp} from "../ConfigFtp";
import {DEVICE_CONNECTION_TYPE, DEVICE_OS_TYPE} from "../Device";
import {FTP_DEFAULT_PORT, RDP_DEFAULT_COLOR_DEPTH, RDP_DEFAULT_KEYBOARD_LAYOUT, RDP_DEFAULT_PORT, SNMP_DEFAULT_COMMUNITY, SNMP_DEFAULT_PORT, SNMP_DEFAULT_RETRIES, SNMP_DEFAULT_TIMEOUT_SECONDS, SNMP_DEFAULT_VERSION, SSH_DEFAULT_PORT, SSH_DEFAULT_TERM, TELNET_DEFAULT_MAX_SESSIONS, TELNET_DEFAULT_PORT, TELNET_DEFAULT_TERM} from "@/common/const/DeviceConst";

export interface ReqDevice {
    device: Device;
    group_id?: string | null;
    config_ssh?: ConfigSsh | null;
    config_rdp?: ConfigRdp | null;
    config_ftp?: ConfigFtp | null;
    config_telnet?: ConfigTelnet | null;
    config_snmp?: ConfigSnmp | null;
}

export function createEmptyReqDevice(): ReqDevice {
    return {
        device: {
            id: String(),
            name: String(),
            host: String(),
            updated_at: 0,
            os_type: DEVICE_OS_TYPE.UNKNOWN,
            type: DEVICE_CONNECTION_TYPE.REMOTE,
        },
        group_id: DEFAULT_GROUP_ID,
        config_ssh: {
            base: {port: SSH_DEFAULT_PORT, term: SSH_DEFAULT_TERM, keepalive_interval_seconds: 0, connect_timeout_seconds: null},
            passphrase: null,
            agent_key_id: null,
            use_gssapi: false,
            use_ssh_agent: false,
            share_session: true,
            preferred_auth_method: null,
            shell_idle_timeout_seconds: 300,
            keyboard_interactive_answers: null,
        },
        config_rdp: {
            domain: null,
            ignore_cert: false,
            desktop_width: null,
            desktop_height: null,
            console_session: false,
            performance_flags: null,
            security: RDP_SECURITY.AUTO,
            color_depth: RDP_DEFAULT_COLOR_DEPTH,
            keyboard_layout: RDP_DEFAULT_KEYBOARD_LAYOUT,
            base: {port: RDP_DEFAULT_PORT, term: String(), keepalive_interval_seconds: 0, connect_timeout_seconds: null},
        },
        config_ftp: {
            ignore_cert: false,
            security: FTP_SECURITY.NONE,
            base: {port: FTP_DEFAULT_PORT, term: String(), keepalive_interval_seconds: 0, connect_timeout_seconds: null},
        },
        config_telnet: {
            nvt_eol: false,
            environment: null,
            more_pattern: null,
            more_response: null,
            terminal_types: null,
            username_pattern: null,
            password_pattern: null,
            prompt_pri_pattern: null,
            prompt_alt_pattern: null,
            disable_paging_command: null,
            max_sessions: TELNET_DEFAULT_MAX_SESSIONS,
            base: {port: TELNET_DEFAULT_PORT, term: TELNET_DEFAULT_TERM, keepalive_interval_seconds: 0, connect_timeout_seconds: null},
        },
        config_snmp: {
            enabled: false,
            port: SNMP_DEFAULT_PORT,
            version: SNMP_DEFAULT_VERSION,
            retries: SNMP_DEFAULT_RETRIES,
            community: SNMP_DEFAULT_COMMUNITY,
            timeout_seconds: SNMP_DEFAULT_TIMEOUT_SECONDS,
        },
    };
}