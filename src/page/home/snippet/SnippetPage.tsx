import {Box} from '@mui/material';
import {TopBar} from './toolbar/TopBar';
import {Content} from './content/Content';
import {EditDialog} from './edit/EditDialog';
import {useCallback, useEffect, useState} from 'react';
import {useSnippetService} from '@/common/context/useAppHooks.tsx';
import {resolveErrorText} from '@/common/model/resp/ResultModel.ts';
import {createEmptySnippet, type Snippet} from '@/common/model/Snippet.ts';

export function SnippetPage(props: {active: boolean}) {
    const {active} = props;
    const snippetService = useSnippetService();
    const [loaded, setLoaded] = useState<boolean>(false);
    const [snippets, setSnippets] = useState<Snippet[]>([]);
    const [error, setError] = useState<string | null>(null);
    const [dialogOpen, setDialogOpen] = useState<boolean>(false);
    const [searchText, setSearchText] = useState<string | null>(null);
    const [snippet, setSnippet] = useState<Snippet>(createEmptySnippet());

    const reload = useCallback((): void => {
        snippetService.selectAll().then(list => {
            setSnippets([...list].sort((a, b) => a.sort - b.sort));
            setError(null);
            setLoaded(true);
        }).catch(err => {
            setLoaded(true);
            setError(resolveErrorText(err));
        });
    }, [snippetService]);

    const handleAdd = (): void => {
        if (dialogOpen) {
            setDialogOpen(false);
            return;
        }
        setSnippet(createEmptySnippet());
        setDialogOpen(true);
    };

    const handleEdit = (value: Snippet): void => {
        setSnippet(value);
        setDialogOpen(true);
    };

    const handleClose = (): void => {
        setDialogOpen(false);
    };

    const handleSaved = (): void => {
        setDialogOpen(false);
        reload();
    };

    useEffect(() => {
        if (!active) {
            return;
        }
        reload();
    }, [active, reload]);

    return (
        <Box className="tm-home-page tm-home-page-column">
            <Box className="tm-agent-page tm-page-column">
                <TopBar searchText={searchText} onSearchChange={setSearchText} onAdd={handleAdd}/>
                <Content loading={!loaded} error={error} snippets={snippets} searchText={searchText} onAdd={handleAdd} onEdit={handleEdit} onDeleted={reload}/>
            </Box>
            <EditDialog open={dialogOpen} snippet={snippet} onSaved={handleSaved} onClose={handleClose}/>
        </Box>
    );
}
