interface SyncDownloadIconProps {
    className?: string;
    fontSize?: number;
}

export function SyncDownloadIcon(props: SyncDownloadIconProps) {
    const {className, fontSize} = props;

    return (
        <svg className={className} viewBox="0 0 24 24" width={fontSize} height={fontSize} fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden>
            <g stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round">
                <path d="M12 6v9"/>
                <path d="m8.5 11.5 3.5 3.5 3.5-3.5"/>
                <path d="M5 18h14"/>
            </g>
        </svg>
    );
}
