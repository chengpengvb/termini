import {useEffect, useState} from 'react';

function readDocumentClass(className: string): boolean {
    return document.documentElement.classList.contains(className);
}

export function useDocumentClass(className: string): boolean {
    const [on, setOn] = useState(() => readDocumentClass(className));

    useEffect(() => {
        const root = document.documentElement;
        const sync = (): void => {
            setOn(readDocumentClass(className));
        };
        sync();
        const observer = new MutationObserver(sync);
        observer.observe(root, {attributes: true, attributeFilter: ['class']});
        return () => {
            observer.disconnect();
        };
    }, [className]);

    return on;
}

export function useSoftKeyboardOpen(): boolean {
    return useDocumentClass('tm-keyboard-open');
}
