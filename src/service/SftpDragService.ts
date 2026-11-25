import type {BaseService} from './BaseService.ts';
import type {SftpDragHandle, SftpDragOptions} from '@/page/common/dto/sftp/SftpDrag.ts';

export interface SftpDragService extends BaseService {
    disposeHandle(controllerId: string): void;
    createHandle(controllerId: string, options: SftpDragOptions): SftpDragHandle;
}
