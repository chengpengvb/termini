import {useId} from 'react';

const WIDTH = 1000;
const HEIGHT = 100;
const BASELINE = 6;

function buildPath(activity: number[]): string {
    if (activity.length === 0) {
        return `M0 ${HEIGHT} L${WIDTH} ${HEIGHT} Z`;
    }
    const step = WIDTH / activity.length;
    const points = activity.map((value, index) => {
        const x = index * step + step / 2;
        const y = HEIGHT - BASELINE - value * (HEIGHT - BASELINE);
        return [x, y] as const;
    });
    let d = `M0 ${HEIGHT} L0 ${points[0][1]}`;
    for (let i = 0; i < points.length - 1; i += 1) {
        const [x0, y0] = points[i];
        const [x1, y1] = points[i + 1];
        const cx = (x0 + x1) / 2;
        d += ` C${cx} ${y0} ${cx} ${y1} ${x1} ${y1}`;
    }
    d += ` L${WIDTH} ${points[points.length - 1][1]} L${WIDTH} ${HEIGHT} Z`;
    return d;
}

export function ActivityWave(props: {activity: number[]; ratio: number}) {
    const {activity, ratio} = props;
    const clipId = useId();
    const path = buildPath(activity);
    const played = Math.max(0, Math.min(1, ratio)) * WIDTH;

    return (
        <svg className="tm-record-wave" viewBox={`0 0 ${WIDTH} ${HEIGHT}`} preserveAspectRatio="none" aria-hidden>
            <defs>
                <clipPath id={clipId}>
                    <rect x={0} y={0} width={played} height={HEIGHT}/>
                </clipPath>
            </defs>
            <path d={path} className="tm-record-wave-rest"/>
            <path d={path} className="tm-record-wave-played" clipPath={`url(#${clipId})`}/>
            <rect x={0} y={HEIGHT - BASELINE} width={WIDTH} height={BASELINE} className="tm-record-wave-rest"/>
            <rect x={0} y={HEIGHT - BASELINE} width={played} height={BASELINE} className="tm-record-wave-played"/>
        </svg>
    );
}
