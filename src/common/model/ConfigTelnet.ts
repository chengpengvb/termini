import type { ConfigTermBase } from "./ConfigTermBase";

export interface ConfigTelnet {
  id?: string;
  nvt_eol: boolean;
  device_id?: string;
  max_sessions?: number;
  base?: ConfigTermBase;
  cmd_verify?: boolean | null;
  more_pattern?: string | null;
  more_response?: string | null;
  terminal_types?: string[] | null;
  username_pattern?: string | null;
  password_pattern?: string | null;
  prompt_pri_pattern?: string | null;
  prompt_alt_pattern?: string | null;
  disable_paging_command?: string | null;
  cmd_verify_timeout_seconds?: number | null;
  environment?: Record<string, string> | null;
}
