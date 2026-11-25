interface ChatIconProps {
    className?: string;
    fontSize?: number;
}

export function ChatIcon(props: ChatIconProps) {
    const {className, fontSize} = props;

    return (
        <svg className={className} viewBox="0 0 24 24" width={fontSize} height={fontSize} fill="currentColor" xmlns="http://www.w3.org/2000/svg" aria-hidden>
            <path d="M6.6 2.4h10.8c3.1 0 4.6 2.1 4.6 5.4v5.4c0 3.3-1.5 5.4-4.6 5.4H9.7L8.2 21.6 6.6 17.8c-3.1 0-4.6-2.1-4.6-5.4V7.8c0-3.3 1.5-5.4 4.6-5.4z"/>
        </svg>
    );
}
