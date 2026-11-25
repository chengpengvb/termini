import type {AiChatItem} from '@/common/model/AiChatItem.ts';

export interface ItemHandle {
    onEvent: (item: AiChatItem) => void;
    replay: (items: AiChatItem[]) => void;
}
