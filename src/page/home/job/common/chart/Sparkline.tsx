import {Box} from '@mui/material';
import type {JobTone} from '@/page/common/util/jobUtil.ts';
import {buildSparklinePoints, CHART_SAMPLE_MIN, type ChartSample} from '@/page/common/util/chartUtil.ts';

interface SparklineProps {
    width?: number;
    height?: number;
    tone?: JobTone;
    samples: ChartSample[];
}

const SPARKLINE_PAD = 2;

export function Sparkline(props: SparklineProps) {
    const {samples, tone = 'neutral', height = 24, width = 64} = props;

    if (samples.length < CHART_SAMPLE_MIN) {
        return <Box className="tm-job-sparkline is-empty" style={{width, height}}/>;
    }

    const points = buildSparklinePoints(samples, width, height, SPARKLINE_PAD);

    return (
        <svg className={`tm-job-sparkline is-${tone}`} width={width} height={height} viewBox={`0 0 ${width} ${height}`} aria-hidden>
            <polyline fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinejoin="round" strokeLinecap="round" points={points}/>
        </svg>
    );
}
