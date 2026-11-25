interface UploadDropIconProps {
    className?: string;
    fontSize?: number;
}

export function UploadDropIcon(props: UploadDropIconProps) {
    const {className, fontSize} = props;

    return (
        <svg className={className} viewBox="0 0 48 48" width={fontSize} height={fontSize} aria-hidden>
            <path d="M24 8v20"/>
            <path d="M17 22l7 7 7-7"/>
            <path d="M12 36h24"/>
            <path d="M12 36v-4"/>
            <path d="M36 36v-4"/>
        </svg>
    );
}
