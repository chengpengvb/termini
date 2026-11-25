import type {BaseService} from './BaseService';
import type {SessionRecord} from '@/common/model/SessionRecord.ts';

export interface SessionRecordService extends BaseService {

    list(): Promise<SessionRecord[]>;

    content(id: string): Promise<string>;

    deleteById(id: string): Promise<void>;

    deleteBefore(startedBeforeMillis: number): Promise<number>;
}
