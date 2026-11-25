import type {BaseService} from './BaseService';
import type {JobDevice} from '@/common/model/JobDevice.ts';
import type {RespMetricItem} from '@/common/model/resp/RespMetricItem.ts';
import type {JobDeviceDrawerTab, JobMonitorView} from '@/page/common/model/JobModel';

export interface JobDeviceService extends BaseService {

    getLivePins(): string[];

    getExpanded(id: string): boolean;

    toggleLivePin(key: string): string[];

    getMonitorView(): JobMonitorView;

    getDrawerTab(): JobDeviceDrawerTab;

    delete(id: string): Promise<number>;

    takeCollectGap(): Promise<number>;

    selectAllDetail(): Promise<JobDevice[]>;

    setMonitorView(view: JobMonitorView): void;

    setDrawerTab(tab: JobDeviceDrawerTab): void;

    saveBatch(deviceIds: string[]): Promise<number>;

    setExpanded(id: string, expanded: boolean): void;

    setEnabled(id: string, enabled: boolean): Promise<boolean>;

    selectMetric(jobDeviceId: string, jobIndicatorIds: string[]): Promise<RespMetricItem[]>;

    addMetricListener(jobDeviceId: string, listener: (metric: RespMetricItem) => void): () => void;

}
