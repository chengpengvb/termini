interface DeckIconProps {
    className?: string;
    fontSize?: number;
}

export function DeckIcon(props: DeckIconProps) {
    const {className, fontSize} = props;

    return (
        <svg className={className} viewBox="0 0 24 24" width={fontSize} height={fontSize} fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden>
            <g stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round">
                <path d="m12 3.5 1.8 5.7 5.7 1.8-5.7 1.8L12 18.5l-1.8-5.7L4.5 11l5.7-1.8z"/>
                <path d="M18.5 4.5v3M20 6h-3M6 16.5v2.5M7.25 17.75H4.75"/>
            </g>
        </svg>
    );
}