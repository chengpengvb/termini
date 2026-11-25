import type {ApiService} from './ApiService';
import type {JobDiscoverDrawerTab} from '@/page/common/model/JobModel';
import type {JobDiscover, JobDiscoverItem} from '@/common/model/JobDiscover.ts';

export interface JobDiscoverService extends ApiService<JobDiscover> {

    getDrawerTab(): JobDiscoverDrawerTab;

    accept(ids: string[]): Promise<void>;

    deleteBatch(ids: string[]): Promise<number>;

    handleCancel(chatId: string): Promise<void>;

    setDrawerTab(tab: JobDiscoverDrawerTab): void;

    selectItem(discoverId: string): Promise<JobDiscoverItem[]>;

    handleStart(jobDeviceId: string): Promise<JobDiscover | null>;

    addJobDiscoverListChangeListener(listener: (list: JobDiscover[]) => void): () => void;

}
