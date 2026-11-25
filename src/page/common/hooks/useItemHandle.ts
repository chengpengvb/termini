import {Ref, useImperativeHandle, useRef} from 'react';
import type {AiChatItem} from '@/common/model/AiChatItem.ts';
import type {ItemHandle} from '@/page/common/dto/agent/ItemHandle.ts';

export function useItemHandle(ref: Ref<ItemHandle> | undefined, onEvent: (item: AiChatItem) => void): void {
    const replayedRef = useRef(false);
    const onEventRef = useRef(onEvent);
    onEventRef.current = onEvent;

    useImperativeHandle(ref, () => ({
        onEvent: (item: AiChatItem): void => {
            onEventRef.current(item);
        },
        replay: (items: AiChatItem[]): void => {
            if (replayedRef.current) {
                return;
            }
            replayedRef.current = true;
            for (const item of items) {
                onEventRef.current(item);
            }
        },
    }), []);
}
