export interface ReqSshShellOpen {
  deviceId: string;
}

export interface ReqSshShellClose {
  deviceId: string;
}

export interface ReqSshShellExec {
  deviceId: string;
  command: string;
  execTimeoutSeconds: number;
}
