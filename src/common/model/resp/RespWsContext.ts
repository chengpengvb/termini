export interface RespSocketContext {
  type: number;
  socketId?: string;
  session_id: string;
  payload?: unknown | null;
}
