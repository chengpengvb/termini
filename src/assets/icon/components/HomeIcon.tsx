interface HomeIconProps {
    className?: string;
    fontSize?: number;
}

export function HomeIcon(props: HomeIconProps) {
    const {className, fontSize} = props;

    return (
        <svg className={className} viewBox="0 0 24 24" width={fontSize} height={fontSize} fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden>
            <g stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round">
                <path d="M4 8.5 12 4l8 4.5-8 4.5L4 8.5z"/>
                <path d="m4 12.5 8 4.5 8-4.5"/>
                <path d="m4 16.5 8 4.5 8-4.5"/>
            </g>
        </svg>
    );
}
