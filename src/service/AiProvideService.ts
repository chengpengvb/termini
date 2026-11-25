import type {ApiService} from './ApiService';
import type {RespProvider} from '@/common/model/resp/RespProvider.ts';
import type {AiProvide, SelectedProvide} from '@/common/model/AiProvide.ts';

export interface AiProvideService extends ApiService<AiProvide> {

    refresh(): Promise<AiProvide[]>;

    getSelectedProvide(): SelectedProvide | null;

    listTemplateProvider(): Promise<RespProvider[]>;

    getThinkingLevel(provideId: string): string | null;

    setSelectedProvide(id: string | null, source: number): void;

    setThinkingLevel(provideId: string, level: string | null): void;

    getTemplateProvider(provideId: number): Promise<RespProvider | null>;

    addAiProvideListChangeListener(listener: (list: AiProvide[]) => void): () => void;
}
