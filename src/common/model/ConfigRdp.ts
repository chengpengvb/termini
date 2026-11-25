import type { ConfigTermBase } from "./ConfigTermBase";

export const RDP_SECURITY = {
  AUTO: 0,
  TLS: 1,
  NLA: 2,
} as const;

export interface ConfigRdp {
  id?: string;
  security: number;
  device_id?: string;
  color_depth: number;
  ignore_cert: boolean;
  domain?: string | null;
  base?: ConfigTermBase;
  keyboard_layout: number;
  console_session: boolean;
  aad_tenant?: string | null;
  drive_path?: string | null;
  desktop_width?: number | null;
  aad_client_id?: string | null;
  desktop_height?: number | null;
  performance_flags?: number | null;
}
