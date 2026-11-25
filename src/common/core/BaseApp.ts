export interface BaseApp {
  getPhase(): number;
  quit(): Promise<void>;
  start(): Promise<void>;
}

