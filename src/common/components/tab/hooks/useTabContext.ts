import {useContext} from 'react';
import {TabContext} from '../context/TabContext';
import type {TabContextValue} from '../context/TabContext';

export function useTabContext(): TabContextValue {
    const ctx = useContext(TabContext);
    if (!ctx) {
        throw new Error('useTabContext must be used inside <TabRoot>');
    }
    return ctx;
}
