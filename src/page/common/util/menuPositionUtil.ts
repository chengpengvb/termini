export interface MenuPosition {
    left: number;
    top: number;
}

export interface MenuPositionClampInput {
    left: number;
    top: number;
    menuWidth: number;
    menuHeight: number;
    viewportWidth: number;
    viewportHeight: number;
    viewportGap: number;
}

export function clampMenuPosition(input: MenuPositionClampInput): MenuPosition {
    const {left, top, menuWidth, menuHeight, viewportWidth, viewportHeight, viewportGap} = input;

    let nextLeft = left;
    let nextTop = top;

    const maxLeft = viewportWidth - menuWidth - viewportGap;
    const maxTop = viewportHeight - menuHeight - viewportGap;

    if (nextLeft > maxLeft) {
        nextLeft = maxLeft;
    }
    if (nextTop > maxTop) {
        nextTop = maxTop;
    }
    if (nextLeft < viewportGap) {
        nextLeft = viewportGap;
    }
    if (nextTop < viewportGap) {
        nextTop = viewportGap;
    }

    return {left: nextLeft, top: nextTop};
}

export interface ClampMenuPositionForElementInput {
    left: number;
    top: number;
    element: HTMLElement;
    viewportGap: number;
}

export function clampMenuPositionForElement(input: ClampMenuPositionForElementInput): MenuPosition {
    const rect = input.element.getBoundingClientRect();
    return clampMenuPosition({
        left: input.left,
        top: input.top,
        menuWidth: rect.width,
        menuHeight: rect.height,
        viewportWidth: window.innerWidth,
        viewportHeight: window.innerHeight,
        viewportGap: input.viewportGap,
    });
}
