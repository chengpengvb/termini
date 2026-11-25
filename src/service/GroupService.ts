import type {ApiService} from './ApiService';
import type {Group} from '@/common/model/Group';
import type {ReqGroupSet} from '@/common/model/req/ReqGroupSet';

export interface GroupService extends ApiService<Group> {
    setDevice2Group(req: ReqGroupSet): Promise<void>;
}
