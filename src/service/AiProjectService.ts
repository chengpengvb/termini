import type {ApiService} from './ApiService';
import type {AiProject} from '@/common/model/AiProject.ts';

export interface AiProjectService extends ApiService<AiProject> {
    create(name: string): Promise<AiProject>;

    rename(id: string, name: string): Promise<void>;

    addAiProjectListChangeListener(listener: (list: AiProject[]) => void): () => void;
}
