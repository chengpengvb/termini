import type {BaseService} from './BaseService';
import type {AiPlanRun, AiPlanTask} from '@/common/model/AiPlan.ts';

export interface AiPlanService extends BaseService {

    runs(): Promise<AiPlanRun[]>;

    selectAll(): Promise<AiPlanTask[]>;

    runNow(id: string): Promise<AiPlanRun>;

    cancelRun(runId: string): Promise<void>;

    deleteRun(runId: string): Promise<void>;

    deleteWithRuns(id: string): Promise<void>;

    save(task: AiPlanTask): Promise<AiPlanTask>;

}
