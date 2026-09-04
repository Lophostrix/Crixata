/**
 * Crixata Chrome Extension - Types Definition
 */

import type {
  Grade,
  GradeSource,
  PolicySummary,
  PolicyType as CachePolicyType,
  UserRights,
  GradeRequest,
  GradeResponse,
} from '../../../packages/cache-schema/types';

export type Summary = PolicySummary;
export type {
  Grade,
  GradeSource,
  PolicySummary,
  CachePolicyType,
  UserRights,
  GradeRequest,
  GradeResponse,
};

export type PolicyType = 'privacy' | 'terms' | 'cookie' | 'unknown';
export type Confidence = 'high' | 'medium' | 'low';

export interface DetectionResult {
  isPolicyPage: boolean;
  policyType: PolicyType;
  confidence: Confidence;
}

export interface ExtractedPageText {
  text: string;
  title: string;
  url: string;
}

export interface CandidateLink {
  text: string;
  href: string;
}

export interface TabGradeRecord {
  tabId: number;
  url: string;
  domain: string;
  title: string;
  isPolicyPage: boolean;
  policyType: PolicyType;
  confidence: Confidence;
  grade?: Grade | '?' | '...';
  summary?: PolicySummary;
  cached?: boolean;
  source?: GradeSource;
  candidateLinks?: CandidateLink[];
  appHealthy?: boolean;
  error?: string;
  timestamp: number;
}
