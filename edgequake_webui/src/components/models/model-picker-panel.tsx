/**
 * Two-step provider → model picker (SPEC-043 wireframe + exceptional UX).
 * Provider select first, then model select filtered to that provider.
 */
"use client";

import { ProviderIcon } from "@/components/providers/provider-icon";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import { Label } from "@/components/ui/label";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { cn } from "@/lib/utils";
import {
  getProviderDisplayName,
  getProviderSubtitle,
} from "@/lib/provider-display";
import { apiClient } from "@/lib/api/client";
import {
  searchHitsToPickerOptions,
  type ModelSearchHit,
} from "@/components/models/model-picker-mappers";
import {
  ensureSelectedInPickerOptions,
  mergePickerOptions,
  providerInCatalog,
} from "@/components/models/model-picker-options";
import { CHOICE_POPOVER } from "@/components/models/choice-menu-surface";
import { Check, ChevronDown, Loader2 } from "lucide-react";
import { useCallback, useEffect, useId, useMemo, useRef, useState } from "react";
import {
  useScrollContainedWheel,
  useScrollSelectedIntoView,
} from "@/hooks/use-scroll-contained-wheel";

export interface ModelPickerValue {
  provider: string;
  model: string;
  fullId: string;
}

export interface ModelPickerOption {
  provider: string;
  providerDisplayName: string;
  name: string;
  displayName: string;
  fullId: string;
  contextLength: number;
  supportsVision: boolean;
  supportsTools: boolean;
  supportsStreaming: boolean;
  /** Embedding models: vector dimension (shown instead of context length). */
  dimension?: number;
  deprecated?: boolean;
  /** Model was discovered via live provider API (Ollama, LM Studio, OpenRouter, …). */
  isLive?: boolean;
  /** Runtime availability from provider discovery. */
  available?: boolean;
}

interface ModelSearchResponse {
  hits: ModelSearchHit[];
  total: number;
}

export interface ModelPickerPanelProps {
  options: ModelPickerOption[];
  value?: ModelPickerValue;
  onChange?: (value: ModelPickerValue | undefined) => void;
  disabled?: boolean;
  className?: string;
  placeholder?: string;
  allowServerDefault?: boolean;
  serverDefaultLabel?: string;
  filterVision?: boolean;
  isLoading?: boolean;
  /** LLM shows capability chips + remote search; embedding shows dimensions only. */
  variant?: "llm" | "embedding";
  /**
   * @deprecated Chip bars removed — provider is a dedicated select. Ignored.
   */
  showProviderFilters?: boolean;
  showCapabilityFilters?: boolean;
  enableRemoteSearch?: boolean;
  testId?: string;
}

type CapabilityFilter = "vision" | "tools" | "streaming";

interface ProviderEntry {
  id: string;
  displayName: string;
  subtitle: string | null;
  count: number;
}

export function formatModelFullId(provider: string, model: string): string {
  return `${provider}/${model}`;
}

export function parseModelFullId(fullId: string): { provider: string; model: string } {
  const slashIndex = fullId.indexOf("/");
  if (slashIndex === -1) {
    return { provider: "unknown", model: fullId };
  }
  return {
    provider: fullId.substring(0, slashIndex),
    model: fullId.substring(slashIndex + 1),
  };
}

function formatOptionSubline(opt: ModelPickerOption, isEmbedding: boolean): string {
  if (isEmbedding && opt.dimension) {
    return `${opt.name} · ${opt.dimension}d`;
  }
  if (opt.contextLength > 0) {
    return `${opt.name} · ${(opt.contextLength / 1000).toFixed(0)}K ctx`;
  }
  return opt.name;
}

function capabilityHint(
  opt: ModelPickerOption,
  filterVision: boolean,
  isEmbedding: boolean,
): string | null {
  if (isEmbedding) return null;
  // Only surface a text hint when the picker is vision-scoped (declutter otherwise).
  if (filterVision && opt.supportsVision) return "Vision";
  return null;
}

export function ModelPickerPanel({
  options,
  value,
  onChange,
  disabled,
  className,
  placeholder = "Search models…",
  allowServerDefault = true,
  serverDefaultLabel = "Server default",
  filterVision = false,
  isLoading = false,
  variant = "llm",
  showProviderFilters: _showProviderFilters = false,
  showCapabilityFilters,
  enableRemoteSearch,
  testId = "model-picker-panel",
}: ModelPickerPanelProps) {
  void _showProviderFilters;
  const isEmbedding = variant === "embedding";
  const showCaps = showCapabilityFilters ?? (!isEmbedding && !filterVision);
  const remoteSearch = enableRemoteSearch ?? !isEmbedding;

  const providerLabelId = useId();
  const modelLabelId = useId();

  const [providerOpen, setProviderOpen] = useState(false);
  const [modelOpen, setModelOpen] = useState(false);
  const [providerSearch, setProviderSearch] = useState("");
  const [search, setSearch] = useState("");
  const [selectedProvider, setSelectedProvider] = useState<string | null>(
    value?.provider ?? null,
  );
  const [capabilityFilters, setCapabilityFilters] = useState<Set<CapabilityFilter>>(
    new Set(),
  );
  const [remoteHits, setRemoteHits] = useState<ModelSearchHit[] | null>(null);
  const [providerCatalogHits, setProviderCatalogHits] = useState<ModelSearchHit[] | null>(
    null,
  );
  const [searchLoading, setSearchLoading] = useState(false);
  const [providerCatalogLoading, setProviderCatalogLoading] = useState(false);

  const providerSearchRef = useRef<HTMLInputElement>(null);
  const modelSearchRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const { onWheel, className: scrollContainedClass } = useScrollContainedWheel();
  useScrollSelectedIntoView(listRef, modelOpen);

  const valueOffered = providerInCatalog(value?.provider, options);
  /** A stored pair outside the eligible catalog is not a selection. */
  const activeProvider =
    selectedProvider && (isLoading || providerInCatalog(selectedProvider, options))
      ? selectedProvider
      : null;

  const buildDisplayOptions = useCallback(
    (
      primary: ModelPickerOption[],
      secondary: ModelPickerOption[] = [],
    ): ModelPickerOption[] =>
      ensureSelectedInPickerOptions(
        mergePickerOptions(primary, secondary),
        valueOffered ? value : undefined,
        options,
      ),
    [options, value, valueOffered],
  );

  // Sync provider from selection (e.g. parent sets value). An unoffered slug never opens the model step.
  useEffect(() => {
    if (!value?.provider) return;
    if (providerInCatalog(value.provider, options)) {
      setSelectedProvider(value.provider);
      return;
    }
    if (!isLoading) {
      setSelectedProvider((current) =>
        current?.toLowerCase() === value.provider.toLowerCase() ? null : current,
      );
    }
  }, [value?.provider, options, isLoading]);

  const providers = useMemo((): ProviderEntry[] => {
    const map = new Map<string, number>();
    for (const opt of options) {
      map.set(opt.provider, (map.get(opt.provider) ?? 0) + 1);
    }
    return Array.from(map.entries())
      .map(([id, count]) => ({
        id,
        displayName: getProviderDisplayName(id),
        subtitle: getProviderSubtitle(id),
        count,
      }))
      .sort((a, b) => a.displayName.localeCompare(b.displayName));
  }, [options]);

  const filteredProviders = useMemo(() => {
    const q = providerSearch.trim().toLowerCase();
    if (!q) return providers;
    return providers.filter(
      (p) =>
        p.displayName.toLowerCase().includes(q) ||
        p.id.toLowerCase().includes(q) ||
        (p.subtitle?.toLowerCase().includes(q) ?? false),
    );
  }, [providers, providerSearch]);

  const modelOpenTicket = useRef(0);
  const handleProviderSelect = useCallback(
    (providerId: string) => {
      setSelectedProvider(providerId);
      setProviderOpen(false);
      setProviderSearch("");
      setSearch("");
      setRemoteHits(null);
      setProviderCatalogHits(null);
      if (value && value.provider !== providerId) {
        onChange?.(undefined);
      }
      // Open the model step next. A choice made before that frame must keep the menu closed.
      const ticket = ++modelOpenTicket.current;
      requestAnimationFrame(() => {
        if (modelOpenTicket.current === ticket) setModelOpen(true);
      });
    },
    [value, onChange],
  );

  const handleUseServerDefault = useCallback(() => {
    setSelectedProvider(null);
    onChange?.(undefined);
    setModelOpen(false);
    setProviderOpen(false);
  }, [onChange]);

  // Dynamic provider-scoped catalog when model popover is open.
  useEffect(() => {
    if (
      !modelOpen ||
      !remoteSearch ||
      !activeProvider ||
      search.trim().length >= 2
    ) {
      setProviderCatalogHits(null);
      return;
    }

    let cancelled = false;
    const timer = setTimeout(async () => {
      setProviderCatalogLoading(true);
      try {
        const params = new URLSearchParams({
          provider: activeProvider,
          limit: "50",
        });
        if (filterVision || capabilityFilters.has("vision")) {
          params.set("requires_vision", "true");
        }
        if (capabilityFilters.has("tools")) {
          params.set("requires_tools", "true");
        }
        const data = await apiClient<ModelSearchResponse>(
          `/models/search?${params.toString()}`,
        );
        if (!cancelled) setProviderCatalogHits(data.hits);
      } catch {
        if (!cancelled) setProviderCatalogHits(null);
      } finally {
        if (!cancelled) setProviderCatalogLoading(false);
      }
    }, 150);

    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [
    modelOpen,
    activeProvider,
    capabilityFilters,
    filterVision,
    remoteSearch,
    search,
  ]);

  useEffect(() => {
    if (!modelOpen || !remoteSearch || !activeProvider) return;
    const q = search.trim();
    if (q.length < 2) {
      setRemoteHits(null);
      return;
    }

    const timer = setTimeout(async () => {
      setSearchLoading(true);
      try {
        const params = new URLSearchParams({
          q,
          fuzzy: "true",
          limit: "40",
          provider: activeProvider,
        });
        if (filterVision || capabilityFilters.has("vision")) {
          params.set("requires_vision", "true");
        }
        if (capabilityFilters.has("tools")) {
          params.set("requires_tools", "true");
        }
        const data = await apiClient<ModelSearchResponse>(
          `/models/search?${params.toString()}`,
        );
        setRemoteHits(data.hits);
      } catch {
        setRemoteHits(null);
      } finally {
        setSearchLoading(false);
      }
    }, 200);

    return () => clearTimeout(timer);
  }, [
    search,
    activeProvider,
    capabilityFilters,
    filterVision,
    modelOpen,
    remoteSearch,
  ]);

  const filteredOptions = useMemo(() => {
    if (!activeProvider) return [];
    let list = options.filter((o) => o.provider === activeProvider);
    if (filterVision) list = list.filter((o) => o.supportsVision);
    if (capabilityFilters.has("vision")) list = list.filter((o) => o.supportsVision);
    if (capabilityFilters.has("tools")) list = list.filter((o) => o.supportsTools);
    if (capabilityFilters.has("streaming")) {
      list = list.filter((o) => o.supportsStreaming);
    }
    const q = search.trim().toLowerCase();
    if (q && !remoteHits) {
      list = list.filter(
        (o) =>
          o.name.toLowerCase().includes(q) ||
          o.displayName.toLowerCase().includes(q),
      );
    }
    return mergePickerOptions(list);
  }, [
    options,
    activeProvider,
    capabilityFilters,
    filterVision,
    search,
    remoteHits,
  ]);

  const displayOptions = useMemo(() => {
    if (!activeProvider) return [];
    if (remoteHits && search.trim().length >= 2) {
      return buildDisplayOptions(
        searchHitsToPickerOptions(remoteHits).filter(
          (o) => o.provider === activeProvider,
        ),
      );
    }
    if (providerCatalogHits) {
      return buildDisplayOptions(
        searchHitsToPickerOptions(providerCatalogHits),
        filteredOptions,
      );
    }
    return buildDisplayOptions(filteredOptions);
  }, [
    remoteHits,
    search,
    providerCatalogHits,
    activeProvider,
    filteredOptions,
    buildDisplayOptions,
  ]);

  const toggleCapability = useCallback((cap: CapabilityFilter) => {
    setCapabilityFilters((prev) => {
      const next = new Set(prev);
      if (next.has(cap)) next.delete(cap);
      else next.add(cap);
      return next;
    });
  }, []);

  const handleModelSelect = (fullId: string) => {
    modelOpenTicket.current += 1;
    const { provider, model } = parseModelFullId(fullId);
    onChange?.({ provider, model, fullId });
    setModelOpen(false);
  };

  const capabilityChoices: CapabilityFilter[] = filterVision
    ? ["vision"]
    : ["vision", "tools", "streaming"];

  const selectedOption = value
    ? options.find((o) => o.fullId === value.fullId) ??
      displayOptions.find((o) => o.fullId === value.fullId)
    : undefined;

  const selectedProviderEntry = activeProvider
    ? providers.find((p) => p.id === activeProvider)
    : undefined;

  const modelDisabled = disabled || isLoading || !activeProvider;
  const usingServerDefault = !value && !selectedProvider;
  /** Empty triggers name what is in effect, matching the "Using …" chip below them. */
  const inheritedLabel = allowServerDefault && usingServerDefault ? serverDefaultLabel : null;

  return (
    <div className={cn("space-y-3", className)} data-testid={testId}>
      {/* Step 1 — Provider */}
      <div className="space-y-1.5">
        <Label id={providerLabelId} className="text-xs text-muted-foreground">
          Provider
        </Label>
        <Popover
          modal
          open={providerOpen}
          onOpenChange={(next) => {
            setProviderOpen(next);
            if (!next) setProviderSearch("");
          }}
        >
          <PopoverTrigger asChild>
            <Button
              variant="outline"
              role="combobox"
              aria-expanded={providerOpen}
              aria-labelledby={providerLabelId}
              disabled={disabled || isLoading}
              className="w-full min-w-0 justify-between gap-2 overflow-hidden font-normal h-10"
              data-testid="model-picker-provider-trigger"
            >
              {isLoading ? (
                <span className="flex items-center gap-2 text-muted-foreground">
                  <Loader2 className="h-4 w-4 animate-spin" />
                  Loading…
                </span>
              ) : selectedProviderEntry ? (
                <span className="flex items-center gap-2 truncate min-w-0">
                  <ProviderIcon providerId={selectedProviderEntry.id} />
                  <span className="truncate font-medium">
                    {selectedProviderEntry.displayName}
                  </span>
                  {selectedProviderEntry.subtitle ? (
                    <span className="text-xs text-muted-foreground truncate min-w-0 hidden sm:inline">
                      · {selectedProviderEntry.subtitle}
                    </span>
                  ) : null}
                </span>
              ) : (
                <span className="text-muted-foreground">{inheritedLabel ?? "Select a provider…"}</span>
              )}
              <ChevronDown className="h-4 w-4 shrink-0 opacity-50" />
            </Button>
          </PopoverTrigger>
          <PopoverContent
            className={cn(CHOICE_POPOVER, "w-[var(--radix-popover-trigger-width)] min-w-[min(100%,22rem)] p-1")}
            align="start"
            onOpenAutoFocus={(event) => {
              event.preventDefault();
              providerSearchRef.current?.focus();
            }}
          >
            <Command shouldFilter={false} loop>
              <CommandInput
                ref={providerSearchRef}
                placeholder="Search providers…"
                value={providerSearch}
                onValueChange={setProviderSearch}
                data-testid="model-picker-provider-search"
              />
              <CommandList
                className={cn("max-h-72", scrollContainedClass)}
                onWheel={onWheel}
                data-testid="model-picker-provider-list"
                aria-label="Providers"
              >
                <CommandEmpty>No providers found.</CommandEmpty>
                <CommandGroup>
                  {filteredProviders.map((p) => (
                    <CommandItem
                      key={p.id}
                      value={p.id}
                      data-testid={`model-picker-provider-option-${p.id}`}
                      onSelect={() => handleProviderSelect(p.id)}
                    >
                      <Check
                        className={cn(
                          "mr-2 h-4 w-4 shrink-0",
                          activeProvider === p.id ? "opacity-100" : "opacity-0",
                        )}
                      />
                      <ProviderIcon providerId={p.id} className="mr-2 shrink-0" />
                      <div className="flex flex-col min-w-0 flex-1">
                        <span className="font-medium truncate">{p.displayName}</span>
                        <span className="text-xs text-muted-foreground truncate">
                          {p.subtitle
                            ? `${p.subtitle} · ${p.count} models`
                            : `${p.count} models`}
                        </span>
                      </div>
                    </CommandItem>
                  ))}
                </CommandGroup>
              </CommandList>
            </Command>
          </PopoverContent>
        </Popover>
      </div>

      {/* Step 2 — Model */}
      <div className="space-y-1.5">
        <Label id={modelLabelId} className="text-xs text-muted-foreground">
          Model
        </Label>
        <Popover
          modal
          open={modelOpen}
          onOpenChange={(next) => {
            if (!activeProvider && next) return;
            setModelOpen(next);
            if (!next) {
              setSearch("");
              setRemoteHits(null);
              setProviderCatalogHits(null);
            }
          }}
        >
          <PopoverTrigger asChild>
            <Button
              variant="outline"
              role="combobox"
              aria-expanded={modelOpen}
              aria-labelledby={modelLabelId}
              disabled={modelDisabled}
              className="w-full min-w-0 justify-between gap-2 overflow-hidden font-normal h-10"
              data-testid={`${testId}-trigger`}
            >
              {isLoading ? (
                <span className="flex items-center gap-2 text-muted-foreground">
                  <Loader2 className="h-4 w-4 animate-spin" />
                  Loading models…
                </span>
              ) : !activeProvider ? (
                <span className="text-muted-foreground">{inheritedLabel ?? "Select a provider first"}</span>
              ) : value && valueOffered && selectedOption ? (
                <span className="flex flex-col items-start truncate min-w-0 text-left">
                  <span className="truncate font-medium w-full">
                    {selectedOption.displayName}
                  </span>
                  <span className="text-xs text-muted-foreground truncate w-full">
                    {formatOptionSubline(selectedOption, isEmbedding)}
                  </span>
                </span>
              ) : value && valueOffered ? (
                <span className="truncate font-medium">{value.model}</span>
              ) : (
                <span className="text-muted-foreground">Select a model…</span>
              )}
              <ChevronDown className="h-4 w-4 shrink-0 opacity-50" />
            </Button>
          </PopoverTrigger>
          <PopoverContent
            className={cn(CHOICE_POPOVER, "w-[var(--radix-popover-trigger-width)] min-w-[min(100%,22rem)] p-1")}
            align="start"
            onOpenAutoFocus={(event) => {
              event.preventDefault();
              modelSearchRef.current?.focus();
            }}
          >
            {showCaps && (
              <div
                className="flex flex-wrap gap-1.5 border-b px-2 py-1.5"
                data-testid="model-picker-capability-bar"
              >
                {capabilityChoices.map((cap) => (
                  <Badge
                    key={cap}
                    variant={capabilityFilters.has(cap) ? "default" : "outline"}
                    className="cursor-pointer capitalize text-xs"
                    data-testid={`model-picker-capability-${cap}`}
                    onClick={() => toggleCapability(cap)}
                  >
                    {cap}
                  </Badge>
                ))}
              </div>
            )}
            <Command shouldFilter={false} loop>
              <CommandInput
                ref={modelSearchRef}
                placeholder={placeholder}
                value={search}
                onValueChange={setSearch}
                data-testid={`${testId}-search`}
              />
              <CommandList
                ref={listRef}
                className={cn("max-h-80 relative", scrollContainedClass)}
                onWheel={onWheel}
                data-testid={`${testId}-list`}
                aria-label="Model search results"
              >
                {(searchLoading || providerCatalogLoading) && (
                  <div
                    className="sticky top-0 z-10 flex items-center justify-center border-b bg-popover/95 py-2 text-sm text-muted-foreground backdrop-blur-sm"
                    data-testid={`${testId}-list-loading`}
                  >
                    <Loader2 className="h-4 w-4 animate-spin mr-2" />
                    {providerCatalogLoading && activeProvider
                      ? `Loading ${getProviderDisplayName(activeProvider)} models…`
                      : "Searching…"}
                  </div>
                )}
                <CommandEmpty>
                  {activeProvider
                    ? `No models for ${getProviderDisplayName(activeProvider)}. Type 2+ characters to search.`
                    : "Select a provider first."}
                </CommandEmpty>
                <CommandGroup>
                  {displayOptions.map((opt) => {
                    const hint = capabilityHint(opt, filterVision, isEmbedding);
                    return (
                      <CommandItem
                        key={opt.fullId}
                        value={opt.fullId}
                        disabled={opt.deprecated}
                        data-testid={`model-picker-option-${opt.fullId.replace(/\//g, "-")}`}
                        onSelect={() => handleModelSelect(opt.fullId)}
                      >
                        <Check
                          className={cn(
                            "mr-2 h-4 w-4 shrink-0",
                            value?.fullId === opt.fullId ? "opacity-100" : "opacity-0",
                          )}
                        />
                        <div className="flex flex-col min-w-0 flex-1">
                          <div className="flex items-center gap-1.5 min-w-0">
                            <span className="font-medium truncate">{opt.displayName}</span>
                            {opt.isLive && (
                              <Badge
                                variant="outline"
                                className="h-4 px-1 text-xs shrink-0"
                                data-testid="model-picker-live-badge"
                              >
                                Live
                              </Badge>
                            )}
                            {hint ? (
                              <span className="text-xs text-muted-foreground shrink-0">
                                {hint}
                              </span>
                            ) : null}
                          </div>
                          <span className="text-xs text-muted-foreground truncate">
                            {formatOptionSubline(opt, isEmbedding)}
                          </span>
                        </div>
                      </CommandItem>
                    );
                  })}
                </CommandGroup>
              </CommandList>
            </Command>
          </PopoverContent>
        </Popover>
      </div>

      {allowServerDefault && (
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className={cn(
            "h-auto px-0 py-0 text-xs text-muted-foreground hover:text-foreground",
            usingServerDefault && "text-foreground font-medium",
          )}
          disabled={disabled || isLoading}
          onClick={handleUseServerDefault}
          data-testid="model-picker-use-server-default"
        >
          {usingServerDefault ? `Using ${serverDefaultLabel}` : `Use ${serverDefaultLabel}`}
        </Button>
      )}
    </div>
  );
}
