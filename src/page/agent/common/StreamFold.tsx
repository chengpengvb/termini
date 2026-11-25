import {Box, ButtonBase, Collapse} from '@mui/material';
import {useEffect, useRef, useState, type ReactNode} from 'react';
import {KeyboardArrowDown, KeyboardArrowRight} from '@mui/icons-material';

interface StreamFoldProps {
    done: boolean;
    scrollKey: string;
    doneLabel: string;
    children: ReactNode;
    streamingLabel: string;
}

export function StreamFold(props: StreamFoldProps) {
    const bodyRef = useRef<HTMLDivElement | null>(null);
    const [userToggled, setUserToggled] = useState(false);
    const [expanded, setExpanded] = useState(!props.done);
    const {done, scrollKey, streamingLabel, doneLabel, children} = props;

    function handleToggleExpanded(): void {
        setUserToggled(true);
        setExpanded((current: boolean) => !current);
    }

    function resolveLabel(): string {
        if (done) {
            return doneLabel;
        }
        return streamingLabel;
    }

    useEffect(() => {
        if (userToggled) {
            return;
        }
        setExpanded(!done);
    }, [done, userToggled]);

    useEffect(() => {
        if (done || !expanded) {
            return;
        }
        const body = bodyRef.current;
        if (!body) {
            return;
        }
        body.scrollTop = body.scrollHeight;
    }, [scrollKey, done, expanded]);

    const rowClassName = expanded ? 'tm-agent-fold-row is-open' : 'tm-agent-fold-row';
    const bodyClassName = done ? 'tm-agent-fold-body' : 'tm-agent-fold-body is-streaming';
    const labelClassName = done ? 'tm-agent-fold-label' : 'tm-agent-fold-label tm-agent-shimmer';

    return (
        <Box className={rowClassName}>
            <ButtonBase disableRipple onClick={handleToggleExpanded} className="tm-agent-fold-toggle">
                <Box component="span" className={labelClassName}>{resolveLabel()}</Box>
                <Box component="span" className="tm-agent-fold-chevron">
                    {expanded ? <KeyboardArrowDown sx={{fontSize: 16}}/> : <KeyboardArrowRight sx={{fontSize: 16}}/>}
                </Box>
            </ButtonBase>
            <Collapse in={expanded} unmountOnExit timeout={0}>
                <Box ref={bodyRef} className={bodyClassName}>
                    {children}
                </Box>
            </Collapse>
        </Box>
    );
}
