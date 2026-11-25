import {Box} from '@mui/material';
import remarkGfm from 'remark-gfm';
import type {Components} from 'react-markdown';
import {useMemo, type ReactElement} from 'react';
import {isNotNil} from '@/common/util/StringUtil.ts';
import {FilePreview} from '@/page/agent/common/FilePreview.tsx';
import ReactMarkdown, {defaultUrlTransform} from 'react-markdown';
import {isChatPreviewFile} from '@/page/common/util/fileTypeUtil.ts';
import {AgentCodeBlock} from '@/page/agent/common/AgentCodeBlock.tsx';
import {openFileEditTab} from '@/page/common/util/fileEditOpenUtil.ts';
import {useSystemUtilService, useTabManagerService} from '@/common/context/useAppHooks.tsx';
import {FILE_URL_PREFIX, matchTextFilePath, remarkFilePath, resolveFileLinkPath} from '@/page/common/util/remarkFilePath.ts';

const RENDER_CACHE_LIMIT = 200;

const renderCache = new Map<string, ReactElement>();

interface AgentMarkdownProps {
    content: string;
    className: string;
}

function isHttpUrl(href: string | undefined): href is string {
    if (!href) {
        return false;
    }
    if (!/^https?:\/\//i.test(href)) {
        return false;
    }
    let parsed: URL;
    try {
        parsed = new URL(href);
    } catch {
        return false;
    }
    return /^[a-z0-9.\-:[\]]+$/i.test(parsed.hostname);
}

function transformUrl(url: string): string {
    if (url.startsWith(FILE_URL_PREFIX) || resolveFileLinkPath(url) !== null) {
        return url;
    }
    return defaultUrlTransform(url);
}

function writeRenderCache(key: string, tree: ReactElement): void {
    if (renderCache.size >= RENDER_CACHE_LIMIT) {
        const oldest = renderCache.keys().next().value;
        if (oldest) {
            renderCache.delete(oldest);
        }
    }
    renderCache.set(key, tree);
}

export function AgentMarkdown(props: AgentMarkdownProps) {
    const {content, className} = props;
    const systemUtilService = useSystemUtilService();
    const tabManagerService = useTabManagerService();

    function resolveCodeText(children: unknown): string {
        return String(children).replace(/\n$/, '');
    }

    function isFencedCode(codeClassName: string | undefined, text: string): boolean {
        if (codeClassName) {
            return true;
        }
        return text.includes('\n');
    }

    function resolveLanguage(codeClassName: string | undefined): string | null {
        if (!codeClassName) {
            return null;
        }
        const match = /language-([\w+-]+)/.exec(codeClassName);
        if (!match) {
            return null;
        }
        return match[1];
    }

    const components = useMemo<Components>(() => ({
        p({children}) {
            return <div className="tm-agent-md-paragraph">{children}</div>;
        },
        pre({children}) {
            return <>{children}</>;
        },
        table({children}) {
            return (
                <div className="tm-agent-md-table-wrap">
                    <table>{children}</table>
                </div>
            );
        },
        code({className: codeClassName, children}) {
            const text = resolveCodeText(children);
            if (!isFencedCode(codeClassName, text)) {
                const filePath = matchTextFilePath(text);
                if (isNotNil(filePath)) {
                    if (isChatPreviewFile(filePath)) {
                        return <FilePreview key={filePath} path={filePath}/>;
                    }
                    return (
                        <code
                            className="tm-agent-md-inline-code tm-agent-md-file"
                            title={filePath}
                            onClick={(event) => {
                                event.preventDefault();
                                event.stopPropagation();
                                openFileEditTab(tabManagerService, systemUtilService, filePath);
                            }}
                        >
                            {text}
                        </code>
                    );
                }
                return <code className="tm-agent-md-inline-code">{text}</code>;
            }
            return <AgentCodeBlock code={text} language={resolveLanguage(codeClassName)}/>;
        },
        a({href, children}) {
            const filePath = resolveFileLinkPath(href);
            if (isNotNil(filePath)) {
                if (isChatPreviewFile(filePath)) {
                    return <FilePreview key={filePath} path={filePath}/>;
                }
                return (
                    <a
                        href={href}
                        className="tm-agent-md-link"
                        title={filePath}
                        onClick={(event) => {
                            event.preventDefault();
                            event.stopPropagation();
                            openFileEditTab(tabManagerService, systemUtilService, filePath);
                        }}
                    >
                        {children}
                    </a>
                );
            }
            if (!isHttpUrl(href)) {
                return <span>{children}</span>;
            }
            return (
                <a
                    href={href}
                    className="tm-agent-md-link"
                    title={href}
                    onClick={(event) => {
                        event.preventDefault();
                        event.stopPropagation();
                        void systemUtilService.openUrl(href).catch((error) => {
                            console.error('[AgentMarkdown] openUrl failed:', error);
                        });
                    }}
                >
                    {children}
                </a>
            );
        },
        img({src, alt}) {
            const filePath = resolveFileLinkPath(typeof src === 'string' ? src : undefined);
            if (filePath && isChatPreviewFile(filePath)) {
                return <FilePreview key={filePath} path={filePath}/>;
            }
            if (filePath) {
                return <a href={typeof src === 'string' ? src : undefined} className="tm-agent-md-link" title={filePath} onClick={event => {
                    event.preventDefault();
                    event.stopPropagation();
                    openFileEditTab(tabManagerService, systemUtilService, filePath);
                }}>{alt ?? filePath}</a>;
            }
            return <img src={src} alt={alt}/>;
        },
    }), [systemUtilService, tabManagerService]);

    function resolveTree(): ReactElement {
        const previewContent = content.includes(FILE_URL_PREFIX) || content.includes('![') || [...content.matchAll(/\.[a-z0-9]{1,12}\b/gi)].some(match => isChatPreviewFile(`preview${match[0]}`));
        const cached = previewContent ? undefined : renderCache.get(content);
        if (isNotNil(cached)) {
            return cached;
        }
        const tree = ReactMarkdown({
            children: content,
            components,
            urlTransform: transformUrl,
            remarkPlugins: [remarkGfm, remarkFilePath],
        });
        if (!previewContent) {
            writeRenderCache(content, tree);
        }
        return tree;
    }

    return (
        <Box className={className}>{resolveTree()}</Box>
    );
}
