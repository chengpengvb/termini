import type { ConfigTermBase } from "./ConfigTermBase";

export const FTP_SECURITY = {
  NONE: 0,
  EXPLICIT: 1,
  IMPLICIT: 2,
} as const;

export interface ConfigFtp {
  id?: string;
  security: number;
  device_id?: string;
  ignore_cert: boolean;
  base?: ConfigTermBase;
}
