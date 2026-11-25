interface SyncUploadIconProps {
    className?: string;
    fontSize?: number;
}

export function SyncUploadIcon(props: SyncUploadIconProps) {
    const {className, fontSize} = props;

    return (
        <svg className={className} viewBox="0 0 24 24" width={fontSize} height={fontSize} fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden>
            <g stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round">
                <path d="M12 15V6"/>
                <path d="m8.5 9.5 3.5-3.5 3.5 3.5"/>
                <path d="M5 18h14"/>
            </g>
        </svg>
    );
}
