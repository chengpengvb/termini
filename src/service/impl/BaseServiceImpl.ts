import {injectable} from 'inversify';
import type {BaseService} from '../BaseService';
import {generateId} from "@/common/util/IdUtil.ts";

@injectable()
export class BaseServiceImpl implements BaseService {
    protected readonly id = generateId();

    getId(): string {
        return this.id;
    }

    getPhase(): number {
        return 200;
    }

    start(): Promise<void> {
        return Promise.resolve();
    }

    quit(): Promise<void> {
        return Promise.resolve();
    }

    onLogout(): void {
    }
}

