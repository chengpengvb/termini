import {ConfigTermBase, createEmptyConfigTermBase} from "./ConfigTermBase";

export interface ConfigSsh {
    id?: string;
    base?: ConfigTermBase;
    use_gssapi: boolean;
    device_id?: string;
    use_ssh_agent: boolean;
    share_session: boolean;
    config_key_pair_id?: string;
    passphrase?: string | null;
    agent_key_id?: string | null;
    jump_device_id?: string | null;
    shell_idle_timeout_seconds: number;
    preferred_auth_method?: number | null;
    keyboard_interactive_answers?: string[] | null;
}


export function createEmptyConfigSsh(): ConfigSsh {
    return {
        use_gssapi: false,
        use_ssh_agent: false,
        share_session: true,
        base: createEmptyConfigTermBase(),
        shell_idle_timeout_seconds: 300,
    };
}