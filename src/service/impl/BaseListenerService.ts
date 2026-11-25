import {injectable} from 'inversify';
import {BaseServiceImpl} from './BaseServiceImpl';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';

@injectable()
export abstract class BaseListenerService extends BaseServiceImpl {
    private listenerMap = new Map<string, Array<(v: never) => void>>();

    protected abstract getNotifyEntityType(): number;

    protected onEntityRefresh(): void {
    }

    protected handleEntityNotifyPayload(payload: unknown): void {
        const notifyType = this.getNotifyEntityType();
        if (notifyType === ENTITY_TYPE.NONE) {
            return;
        }
        if (payload !== notifyType) {
            return;
        }
        this.onEntityRefresh();
    }

    protected addListener<T>(key: string, cb: (v: T) => void): () => void {
        if (!this.listenerMap.has(key)) {
            this.listenerMap.set(key, []);
        }
        const arr = this.listenerMap.get(key)!;
        const fn = cb as (v: never) => void;
        arr.push(fn);
        return () => {
            const idx = arr.indexOf(fn);
            if (idx < 0) {
                return;
            }
            arr.splice(idx, 1);
        };
    }

    protected notifyListeners<T>(key: string, value: T): void {
        const arr = this.listenerMap.get(key);
        if (!arr) {
            return;
        }
        for (const fn of arr) {
            (fn as (v: T) => void)(value);
        }
    }
}
