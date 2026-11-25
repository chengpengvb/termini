import Slide from '@mui/material/Slide';
import type { ReactElement, Ref } from 'react';
import { forwardRef, useLayoutEffect, useRef } from 'react';
import type { TransitionProps } from '@mui/material/transitions';

export const SheetTransition = forwardRef(function SheetTransition(props: TransitionProps & { children: ReactElement }, ref: Ref<unknown>) {
  const { in: inProp, onExited, ...other } = props;
  const onExitedRef = useRef(onExited);
  onExitedRef.current = onExited;

  useLayoutEffect(() => {
    if (inProp) {
      return;
    }
    onExitedRef.current?.(document.body);
  }, [inProp]);

  return (
    <Slide
      direction="up"
      ref={ref}
      {...other}
      in={inProp}
      unmountOnExit
      timeout={{ enter: 340, exit: 280 }}
      easing={{
        enter: 'cubic-bezier(0.32, 0.72, 0, 1)',
        exit: 'cubic-bezier(0.4, 0, 0.2, 1)',
      }}
    />
  );
});
