import {injectable} from "inversify";
import {AuthService} from "@/service/AuthService.ts";
import {BaseServiceImpl} from "@/service/impl/BaseServiceImpl.ts";
import {StepEvent} from "@/page/common/dto/terminal/StepEvent.ts";
import {RespSocketContext} from "@/common/model/resp/RespWsContext.ts";
import {OpenStep} from "@/common/model/resp/event/RespEventOpenStep.ts";

@injectable()
export class AuthServiceImpl extends BaseServiceImpl implements AuthService {

    private readonly connectStepBufferLimit = 32;
    protected readonly connectStepBuffer: Map<string, StepEvent[]> = new Map();
    protected readonly connectStepListeners: Map<string, Set<(stepEvent: StepEvent) => void>> = new Map();
    protected readonly connectAuthListeners: Map<string, Set<(context: RespSocketContext) => void>> = new Map();

    notifyConnectStep(session_id: string, stepEvent: StepEvent): void {
        this.appendConnectStep(session_id, stepEvent);
        this.emitEvent(this.connectStepListeners, session_id, stepEvent);
    }

    clearConnectSteps(session_id: string): void {
        this.connectStepBuffer.delete(session_id);
    }

    addConnectStepListener(session_id: string, cb: (stepEvent: StepEvent) => void): () => void {
        const off = this.addSessionListener(this.connectStepListeners, session_id, cb);
        const buffered = this.connectStepBuffer.get(session_id);
        if (buffered) {
            for (const event of buffered) {
                cb(event);
            }
        }
        return off;
    }

    addAuthListener(session_id: string, cb: (context: RespSocketContext) => void): () => void {
        return this.addSessionListener(this.connectAuthListeners, session_id, cb);
    }

    onOpenStep(context: RespSocketContext, isAuthType: (type: number) => boolean): void {
        const step = context.payload as OpenStep;
        if (!isAuthType(step.step_type)) {
            return;
        }
        const set = this.connectAuthListeners.get(context.session_id);
        if (!set) {
            return;
        }
        for (const cb of set) {
            cb(context);
        }
    }

    public addSessionListener<TCb extends (...args: any[]) => void>(store: Map<string, Set<TCb>>, session_id: string, cb: TCb): () => void {
        const set = store.get(session_id);
        if (set) {
            set.add(cb);
        } else {
            store.set(session_id, new Set([cb]));
        }
        return () => {
            const current = store.get(session_id);
            if (!current) {
                return;
            }
            current.delete(cb);
            if (current.size === 0) {
                store.delete(session_id);
            }
        };
    }

    emitEvent<TArg>(store: Map<string, Set<(arg: TArg) => void>>, session_id: string, arg: TArg): void {
        const set = store.get(session_id);
        if (!set) {
            return;
        }
        for (const cb of set) {
            cb(arg);
        }
    }

    private appendConnectStep(session_id: string, stepEvent: StepEvent): void {
        const current = this.connectStepBuffer.get(session_id);
        if (!current) {
            this.connectStepBuffer.set(session_id, [stepEvent]);
            return;
        }
        current.push(stepEvent);
        if (current.length <= this.connectStepBufferLimit) {
            return;
        }
        current.splice(0, current.length - this.connectStepBufferLimit);
    }

}