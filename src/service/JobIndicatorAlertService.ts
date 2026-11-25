import type {BaseService} from './BaseService';
import type {JobIndicatorAlert} from '@/common/model/JobIndicatorAlert.ts';

export interface JobIndicatorAlertService extends BaseService {

    deleteBatch(ids: string[]): Promise<number>;

    selectRecent(limit: number): Promise<JobIndicatorAlert[]>;

    addAlertListChangeListener(listener: (alertId: string) => void): () => void;

    selectList(jobDeviceId: string, jobIndicatorId: string): Promise<JobIndicatorAlert[]>;
}
