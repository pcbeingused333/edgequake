"use client";

/**
 * SPEC-175 — one map from section id to its component. The shell renders the
 * section for the current route; nothing else decides what a section shows.
 */

import { AccessSection } from "@/components/settings/sections/access-section";
import { AppearanceSection } from "@/components/settings/sections/appearance-section";
import { BrowserDataSection } from "@/components/settings/sections/browser-data-section";
import { ConnectionsSection } from "@/components/settings/sections/connections-section";
import { DeploymentProvidersSection } from "@/components/settings/sections/deployment-providers-section";
import { FleetSection } from "@/components/settings/sections/fleet-section";
import { GraphSection } from "@/components/settings/sections/graph-section";
import { IngestionSection } from "@/components/settings/sections/ingestion-section";
import { ObservabilitySection } from "@/components/settings/sections/observability-section";
import { OrganizationProvidersSection } from "@/components/settings/sections/organization-providers-section";
import { PeopleSection } from "@/components/settings/sections/people-section";
import { SecretsSection } from "@/components/settings/sections/secrets-section";
import { WorkspaceModelsSection } from "@/components/settings/sections/workspace-models-section";
import type { SettingsSectionId } from "@/lib/settings/registry";
import type { ComponentType } from "react";

const SECTION_COMPONENTS: Record<SettingsSectionId, ComponentType> = {
  appearance: AppearanceSection,
  graph: GraphSection,
  "browser-data": BrowserDataSection,
  models: WorkspaceModelsSection,
  ingestion: IngestionSection,
  access: AccessSection,
  people: PeopleSection,
  "org-providers": OrganizationProvidersSection,
  secrets: SecretsSection,
  connections: ConnectionsSection,
  providers: DeploymentProvidersSection,
  fleet: FleetSection,
  observability: ObservabilitySection,
};

export function SettingsSectionRenderer({ id }: { id: SettingsSectionId; workspaceSlug?: string | null }) {
  const Section = SECTION_COMPONENTS[id];
  return <Section />;
}
