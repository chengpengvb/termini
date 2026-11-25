import type {ApiService} from './ApiService';
import type {IndexDirStatus} from '@/common/model/resp/RespCodeIndex.ts';

export interface CodeIndexService extends ApiService<IndexDirStatus> {

    status(): Promise<IndexDirStatus[]>;

    rebuild(dir: string): Promise<void>;

    saveDirs(dirs: string[]): Promise<string[]>;

}
