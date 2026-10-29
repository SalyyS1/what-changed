import type {
  OwnerCapabilityDto, OwnerChangeDraftDto, OwnerRuleRevisionDto, ScenarioRevisionDto,
  OwnerEvidenceReviewDto, OwnerScenarioEvidenceDto, OwnerCandidateRevisionDto,
} from '../tauri/bindings';

export type OwnerNodeKind = 'capability' | 'rule' | 'scenario' | 'evidence';
export type OwnerMapMode = '2d' | '3d' | 'list';
export type OwnerLayer = 'intent' | 'scenarios' | 'evidence';
export interface OwnerGraphNode {
  readonly id: string;
  readonly kind: OwnerNodeKind;
  readonly layer: OwnerLayer;
  readonly label: string;
  readonly capabilityId: string;
  readonly revision: number | null;
  readonly status: string | null;
  readonly freshness: string | null;
  readonly selectedChange: boolean;
  readonly gaps: readonly string[];
  readonly source: OwnerCapabilityDto | OwnerRuleRevisionDto | ScenarioRevisionDto | OwnerScenarioEvidenceDto;
  readonly review: OwnerEvidenceReviewDto | null;
}
export interface OwnerGraphEdge {
  readonly id: string;
  readonly source: string;
  readonly target: string;
  readonly kind: 'requires' | 'verifies' | 'observed_by';
}
export interface OwnerGraphInput {
  readonly appId: string;
  readonly capabilities: readonly OwnerCapabilityDto[];
  readonly rules: readonly OwnerRuleRevisionDto[];
  readonly scenarios: readonly ScenarioRevisionDto[];
  readonly reviews: readonly OwnerEvidenceReviewDto[];
  readonly change?: OwnerChangeDraftDto | null;
  readonly candidates?: readonly OwnerCandidateRevisionDto[];
}
export interface OwnerGraph {
  readonly appId: string;
  readonly nodes: readonly OwnerGraphNode[];
  readonly edges: readonly OwnerGraphEdge[];
  readonly reviews: readonly OwnerEvidenceReviewDto[];
  readonly change: OwnerChangeDraftDto | null;
  readonly gaps: readonly string[];
}
export interface OwnerGraphWindow {
  readonly nodes: readonly OwnerGraphNode[];
  readonly edges: readonly OwnerGraphEdge[];
  readonly totalNodes: number;
  readonly totalEdges: number;
  readonly page: number;
  readonly pages: number;
  readonly reason: string | null;
}
export const capabilityNodeId = (id: string): string => `capability:${id}`;
export const ruleNodeId = (id: string, revision: number): string => `rule:${id}:${revision}`;
export const scenarioNodeId = (id: string, revision: number): string => `scenario:${id}:${revision}`;
export const evidenceNodeId = (run: string, id: string, revision: number): string => `evidence:${run}:${id}:${revision}`;
