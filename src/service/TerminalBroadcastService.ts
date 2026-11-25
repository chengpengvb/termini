import type { BaseService } from './BaseService';
import type { BroadcastTerminal } from '@/common/model/dto/BroadcastTerminal.ts';

export interface TerminalBroadcastService extends BaseService {

    isTarget(paneId: string): boolean;

    getTargets(tabId: string): string[];

    listTerminals(): BroadcastTerminal[];

    addChangeListener(cb: () => void): () => void;

    setTargets(tabId: string, active: boolean): void;

    dispatch(sourcePaneId: string, data: string): void;

    register(paneId: string, label: string, input: (data: string) => void): () => void;

}
