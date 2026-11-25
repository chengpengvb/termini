import type {Ref} from 'react';
import {Box} from '@mui/material';
import type {RdpCursorImage} from '@/page/common/hooks/useRdpCursor.ts';
import {RDP_CURSOR_KIND} from '@/common/model/resp/event/RespEventRdp.ts';

export interface RdpPointerProps {
    kind: number;
    visible: boolean;
    ref: Ref<HTMLDivElement>;
    image: RdpCursorImage | null;
}

export function RdpPointer(props: RdpPointerProps) {
    const {kind, visible, ref, image} = props;
    const className = !visible || kind === RDP_CURSOR_KIND.HIDDEN ? 'tm-rdp-cursor is-hidden' : 'tm-rdp-cursor';
    return (
        <Box ref={ref} className={className} style={{transform: 'translate3d(var(--tm-rdp-cursor-x), var(--tm-rdp-cursor-y), 0)'}}>
            {image ? (
                <img src={image.url} width={image.width} height={image.height} alt="" draggable={false} style={{transform: `translate(${-image.hotspotX}px, ${-image.hotspotY}px)`}}/>
            ) : (
                <svg viewBox="0 0 12 20" xmlns="http://www.w3.org/2000/svg">
                    <path d="M1 1v15.6l3.7-3.3 2.5 5.7 2.4-1-2.5-5.7H12z" fill="#fff" stroke="#000" strokeWidth="1" strokeLinejoin="round"/>
                </svg>
            )}
        </Box>
    );
}
