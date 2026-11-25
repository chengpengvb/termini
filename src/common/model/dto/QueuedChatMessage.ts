import type {AttachedImage} from '@/page/common/util/imageAttach.ts';

export interface QueuedChatMessage {
    id: string;
    message: string;
    images: AttachedImage[];
}
