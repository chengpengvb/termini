import { Box } from '@mui/material';
import type { ThemedToken } from 'shiki';
import { isNil, isNotNil } from '@/common/util/StringUtil.ts';
import { renderTokens } from '@/page/agent/common/AgentCodeLines.tsx';
import { resolveLanguageByPath } from '@/page/common/util/codeHighlight.ts';
import { useHighlightTokens } from '@/page/common/hooks/useHighlightTokens.ts';
import type { ReplaceConflictView } from '@/page/common/dto/agent/ToolView.ts';

interface ReplaceDiffViewProps {
  args: ReplaceConflictView;
}

interface DiffLine {
  kind: number;
  text: string;
  line: number | null;
  tokens: ThemedToken[] | undefined;
}

const DIFF_LINE = {
  DEL: 0,
  ADD: 1,
} as const;

function splitLines(text: string): string[] {
  return text ? text.split('\n') : [];
}

function alignTokens(tokens: ThemedToken[][] | null, count: number): ThemedToken[][] | null {
  if (isNil(tokens) || tokens.length !== count) {
    return null;
  }
  return tokens;
}

export function ReplaceDiffView(props: ReplaceDiffViewProps) {
  const { args } = props;
  const language = resolveLanguageByPath(args.fullPath ?? args.path);
  const oldLines = splitLines(args.old_string);
  const newLines = splitLines(args.new_string);
  const oldTokens = alignTokens(useHighlightTokens(args.old_string || null, language), oldLines.length);
  const newTokens = alignTokens(useHighlightTokens(args.new_string || null, language), newLines.length);

  const lines: DiffLine[] = [
    ...oldLines.map((text, index) => ({ kind: DIFF_LINE.DEL, text, line: isNil(args.startLine) ? null : args.startLine + index, tokens: oldTokens?.[index] })),
    ...newLines.map((text, index) => ({ kind: DIFF_LINE.ADD, text, line: isNil(args.startLine) ? null : args.startLine + index, tokens: newTokens?.[index] })),
  ];
  const showLineNumbers = lines.some(line => isNotNil(line.line));

  return (
    <Box className="tm-agent-diff">
      <Box className="tm-agent-diff-head">
        <Box className="tm-agent-diff-file">{args.displayName}</Box>
        <Box className="tm-agent-diff-stats">
          {newLines.length > 0 ? (
            <Box component="span" className="tm-agent-diff-stat-add">
              +{newLines.length}
            </Box>
          ) : null}
          {oldLines.length > 0 ? (
            <Box component="span" className="tm-agent-diff-stat-del">
              -{oldLines.length}
            </Box>
          ) : null}
        </Box>
      </Box>
      <Box className="tm-agent-diff-body">
        {lines.map((line, index) => (
          <Box key={`${line.kind}-${index}`} className={line.kind === DIFF_LINE.DEL ? 'tm-agent-diff-line is-del' : 'tm-agent-diff-line is-add'}>
            {showLineNumbers ? (
              <Box component="span" className="tm-agent-diff-lineno">
                {isNil(line.line) ? '' : line.line}
              </Box>
            ) : null}
            <Box component="span" className="tm-agent-diff-gutter">
              {line.kind === DIFF_LINE.DEL ? '-' : '+'}
            </Box>
            <Box component="span" className="tm-agent-diff-text">
              {renderTokens(line.tokens, line.text)}
            </Box>
          </Box>
        ))}
      </Box>
    </Box>
  );
}
