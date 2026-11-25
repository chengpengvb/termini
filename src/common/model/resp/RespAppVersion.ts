export interface AboutInfo {
    version: string;
    platform: string;
    websiteUrl?: string | null;
    privacyUrl?: string | null;
    termsUrl?: string | null;
}

export type AboutSiteKind = 'website' | 'privacy' | 'terms';
