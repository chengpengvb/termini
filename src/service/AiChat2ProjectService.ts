import type {BaseService} from './BaseService';
import type {AiChat2Project} from '@/common/model/AiChat2Project.ts';

export interface AiChat2ProjectService extends BaseService {
    list(): Promise<AiChat2Project[]>;

    unset(chatId: string): Promise<void>;

    resolveProjectId(chatId: string): string | null;

    setProject(chatId: string, projectId: string): Promise<void>;

    addAiChat2ProjectListChangeListener(listener: (list: AiChat2Project[]) => void): () => void;
}
