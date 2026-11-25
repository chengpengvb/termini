import Grow from '@mui/material/Grow';
import type { ReactElement, Ref } from 'react';
import { forwardRef, useLayoutEffect, useRef } from 'react';
import type { TransitionProps } from '@mui/material/transitions';

const EASING = {
  enter: 'cubic-bezier(0.32, 0.72, 0, 1)',
  exit: 'cubic-bezier(0.4, 0, 0.2, 1)',
};

export const MENU_TRANSITION_DURATION = {
  enter: 200,
  exit: 150,
};

export const MenuTransition = forwardRef(function MenuTransition(props: TransitionProps & { children: ReactElement<unknown, any> }, ref: Ref<HTMLElement>) {
  const { style, easing, in: inProp, onExited, ...other } = props;
  const onExitedRef = useRef(onExited);
  onExitedRef.current = onExited;

  useLayoutEffect(() => {
    if (inProp) {
      return;
    }
    onExitedRef.current?.(document.body);
  }, [inProp]);

  return (
    <Grow
      ref={ref}
      {...other}
      in={inProp}
      unmountOnExit
      easing={easing ?? EASING}
      style={{
        transformOrigin: 'top center',
        ...style,
      }}
    />
  );
});
