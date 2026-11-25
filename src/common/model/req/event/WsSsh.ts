import type { WindowSize } from "../../dto/WindowSize.ts";

/** 与后端 `SshAuthAnswer::from_code` 一一对应 */
export const SSH_AUTH_ANSWER = {
  USERNAME_UPDATED: 0,
  PASSWORD_UPDATED: 1,
  PASSPHRASE_UPDATED: 2,
  KEY_PAIR_UPDATED: 3,
  METHOD_SELECT_UPDATED: 4,
  KEYBOARD_INTERACTIVE_UPDATED: 5,
  AGENT_KEY_SELECT_UPDATED: 6,
  RESELECT_REQUESTED: 7,
  HOST_FINGERPRINT_REJECT: 8,
  HOST_FINGERPRINT_TRUST_ONCE: 9,
  HOST_FINGERPRINT_TRUST_ALWAYS: 10,
} as const;

export interface SshOpen {
  size: WindowSize;
  deviceId: string;
}

export interface SshOpenEvent {
  data: SshOpen;
}

export interface SshAuthAnswerData {
  code: number;
  request_id: string;
}

export interface SshAuthAnswerEvent {
  data: SshAuthAnswerData;
}
