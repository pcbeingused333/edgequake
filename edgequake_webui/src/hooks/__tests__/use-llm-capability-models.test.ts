import { describe, expect, it } from "vitest";
import { llmCapabilityModelsFromConfig } from "@/hooks/use-llm-capability-models";
import type { ModelsListResponse } from "@/lib/api/models";
import { supportedReasoningEffortsForModel } from "@/lib/settings/reasoning-effort-supported";

/** Shape of a nested card on GET /models: provider lives on the parent, not the card. */
function nestedCard(name: string) {
  return {
    name,
    display_name: name,
    model_type: "llm" as const,
    description: "",
    deprecated: false,
    capabilities: {
      context_length: 128000,
      max_output_tokens: 8192,
      supports_vision: false,
      supports_function_calling: true,
      supports_json_mode: true,
      supports_streaming: true,
      supports_system_message: true,
      embedding_dimension: 0,
      supports_thinking: true,
      reasoning_effort: {
        supported: ["low", "high"],
        lowest_structured: "low",
      },
    },
    tags: [] as string[],
  };
}

describe("llmCapabilityModelsFromConfig", () => {
  it("stamps the parent provider onto nested /models cards", () => {
    const data = {
      providers: [
        {
          name: "ollama",
          display_name: "Ollama",
          provider_type: "ollama",
          enabled: true,
          priority: 1,
          description: "",
          models: [nestedCard("gemma4:latest")],
        },
      ],
      default_llm_provider: "ollama",
      default_llm_model: "gemma4:latest",
      default_embedding_provider: "ollama",
      default_embedding_model: "embeddinggemma:latest",
    } as unknown as ModelsListResponse;

    const models = llmCapabilityModelsFromConfig(data);
    expect(models?.[0]?.provider).toBe("ollama");
    expect(models?.[0]?.provider_display_name).toBe("Ollama");
    expect(
      supportedReasoningEffortsForModel(models, "ollama", "gemma4:latest"),
    ).toEqual(["low", "high"]);
  });

  it("returns undefined when the config has not loaded", () => {
    expect(llmCapabilityModelsFromConfig(undefined)).toBeUndefined();
  });
});
