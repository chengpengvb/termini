import type {ApiService} from './ApiService';
import type {RespFileEditRead} from '@/common/model/resp/RespFileEdit.ts';

export interface FileEditService extends ApiService<RespFileEditRead> {
    read(path: string): Promise<RespFileEditRead>;

    write(path: string, content: string, create?: boolean): Promise<void>;
}
