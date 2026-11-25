import {Box} from '@mui/material';
import {createPortal} from 'react-dom';
import {useI18n} from '@/common/i18n/useI18n.ts';
import InsertDriveFile from '@mui/icons-material/InsertDriveFile';
import type {SftpDragPreviewHandle} from '@/page/common/dto/sftp/SftpDrag.ts';
import {forwardRef, useImperativeHandle, useLayoutEffect, useRef, useState} from 'react';

interface PreviewState {
    count: number;
    targetPath: string | undefined;
}

interface PreviewSize {
    width: number;
    height: number;
}

export const BodyDragPreview = forwardRef<SftpDragPreviewHandle, object>(function BodyDragPreview(_, ref) {
    const {t, tf} = useI18n();
    const rafRef = useRef<number | null>(null);
    const elRef = useRef<HTMLDivElement | null>(null);
    const previewSizeRef = useRef<PreviewSize | null>(null);
    const [state, setState] = useState<PreviewState | null>(null);
    const [previewSize, setPreviewSize] = useState<PreviewSize | null>(null);
    const posRef = useRef<{clientX: number; clientY: number}>({clientX: 0, clientY: 0});

    const scheduleMove = (): void => {
        if (rafRef.current) {
            return;
        }
        rafRef.current = requestAnimationFrame(() => {
            rafRef.current = null;
            const el = elRef.current;
            const size = previewSizeRef.current;
            if (!el || !size) {
                return;
            }
            const {clientX, clientY} = posRef.current;
            el.style.transform = `translate3d(${clientX - size.width / 2}px, ${clientY - size.height / 2}px, 0)`;
        });
    };

    useLayoutEffect(() => {
        if (!state || previewSize) {
            return;
        }
        const el = elRef.current;
        if (!el) {
            return;
        }
        const size = {width: el.offsetWidth, height: el.offsetHeight};
        previewSizeRef.current = size;
        setPreviewSize(size);
    }, [state, previewSize]);

    useLayoutEffect(() => {
        if (!previewSize) {
            return;
        }
        scheduleMove();
    }, [previewSize]);

    useImperativeHandle(ref, () => ({
        show(clientX, clientY, count, targetPath) {
            posRef.current = {clientX, clientY};
            setState({count, targetPath});
            scheduleMove();
        },
        move(clientX, clientY) {
            posRef.current = {clientX, clientY};
            scheduleMove();
        },
        hide() {
            if (rafRef.current) {
                cancelAnimationFrame(rafRef.current);
                rafRef.current = null;
            }
            setPreviewSize(null);
            previewSizeRef.current = null;
            setState(null);
        },
    }));

    if (!state) {
        return null;
    }

    const resolveTargetFolderName = (): string => {
        const path = state.targetPath;
        if (!path) {
            return t('protocol.sftp.targetFolder');
        }
        const segments = path.split('/').filter(s => s.length > 0);
        const lastSegment = segments[segments.length - 1];
        if (!lastSegment) {
            return t('protocol.sftp.targetFolder');
        }
        return lastSegment;
    };

    return createPortal(
        <Box ref={elRef} sx={{position: 'fixed', left: 0, top: 0, pointerEvents: 'none', zIndex: 9998, willChange: 'transform'}}>
            <Box className="tm-sftp-drag-preview">
                <Box className="tm-sftp-drag-preview-icon">
                    <InsertDriveFile sx={{fontSize: 24}}/>
                </Box>
                <Box className="tm-sftp-drag-preview-label">
                    {tf('protocol.sftp.dragPreview', {count: state.count, target: resolveTargetFolderName()})}
                </Box>
            </Box>
        </Box>,
        document.body
    );
});
