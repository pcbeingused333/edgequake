"use client";

/**
 * SPEC-175 — settings search. Matches section labels and keywords after the role filter
 * (EC-175-33, EC-175-43), so a hidden section can never be a result.
 *
 * ARIA combobox pattern: ArrowDown/ArrowUp move the active option, Enter opens it,
 * Escape clears the query. `/` focuses the field from anywhere in settings.
 */

import { nextActiveOption } from "@/lib/settings/keyboard";
import { cn } from "@/lib/utils";
import {
  getScope,
  searchSections,
  sectionHref,
  type SettingsSectionDef,
} from "@/lib/settings/registry";
import type { SettingsAccess } from "@/lib/settings/access";
import { Search, X } from "lucide-react";
import { useRouter } from "next/navigation";
import {
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";

export function SettingsSearch({
  access,
  workspaceSlug,
  className,
}: {
  access: SettingsAccess;
  workspaceSlug: string | null;
  className?: string;
}) {
  const router = useRouter();
  const inputRef = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(-1);
  const listId = useId();
  const results = useMemo(() => searchSections(access, query), [access, query]);
  const showResults = query.trim().length > 0;

  // Focus the field with `/` unless the user is already typing somewhere.
  useEffect(() => {
    const onKey = (event: globalThis.KeyboardEvent) => {
      if (event.key !== "/" || event.metaKey || event.ctrlKey || event.altKey)
        return;
      const target = event.target as HTMLElement | null;
      if (
        target &&
        (target.isContentEditable ||
          /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName))
      )
        return;
      const input = inputRef.current;
      // Only the copy in the visible layout (sidebar or mobile header) takes the shortcut.
      if (!input || input.offsetParent === null) return;
      event.preventDefault();
      input.focus();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const open = (section: SettingsSectionDef) => {
    setQuery("");
    setActive(-1);
    router.push(sectionHref(section.id, workspaceSlug));
  };

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Escape") {
      setQuery("");
      setActive(-1);
      inputRef.current?.blur();
      return;
    }
    if (!showResults) return;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      setActive((current) =>
        nextActiveOption(current, results.length, event.key),
      );
      return;
    }
    if (event.key === "Enter") {
      const hit = results[active >= 0 ? active : 0];
      if (hit) {
        event.preventDefault();
        open(hit.section);
      }
    }
  };

  const activeId =
    active >= 0 && results[active]
      ? `${listId}-${results[active].section.id}`
      : undefined;

  return (
    <div className={className}>
      <div className="relative">
        <Search
          className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground"
          aria-hidden="true"
        />
        <input
          ref={inputRef}
          type="search"
          role="combobox"
          aria-label="Search settings"
          aria-expanded={showResults}
          aria-controls={listId}
          aria-autocomplete="list"
          aria-activedescendant={activeId}
          autoComplete="off"
          spellCheck={false}
          placeholder="Search settings"
          value={query}
          onChange={(event) => {
            setQuery(event.target.value);
            setActive(-1);
          }}
          onKeyDown={onKeyDown}
          data-testid="settings-search"
          className={cn(
            "h-8 w-full rounded-md border bg-background pl-8 pr-14 text-sm text-foreground",
            "placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
            "[&::-webkit-search-cancel-button]:hidden",
          )}
        />
        {query ? (
          <button
            type="button"
            aria-label="Clear search"
            onClick={() => {
              setQuery("");
              setActive(-1);
              inputRef.current?.focus();
            }}
            className="absolute right-7 top-1/2 -translate-y-1/2 rounded p-0.5 text-muted-foreground hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          >
            <X className="h-3.5 w-3.5" aria-hidden="true" />
          </button>
        ) : (
          <kbd
            aria-hidden="true"
            className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 rounded border bg-muted px-1 font-mono text-[10px] text-muted-foreground"
          >
            /
          </kbd>
        )}
      </div>

      {showResults ? (
        <div className="mt-1.5">
          {results.length > 0 ? (
            <ul
              id={listId}
              role="listbox"
              aria-label="Matching settings"
              className="flex flex-col gap-0.5"
              data-testid="settings-search-results"
            >
              {results.map((hit, index) => (
                <li
                  key={hit.section.id}
                  id={`${listId}-${hit.section.id}`}
                  role="option"
                  aria-selected={index === active}
                  onMouseDown={(event) => event.preventDefault()}
                  onClick={() => open(hit.section)}
                  className={cn(
                    "cursor-pointer rounded-md px-2 py-1.5 text-sm",
                    index === active
                      ? "bg-accent text-accent-foreground"
                      : "text-foreground hover:bg-muted",
                  )}
                >
                  <span className="font-medium">{hit.section.label}</span>
                  <span className="ml-2 text-xs text-muted-foreground">
                    {getScope(hit.section.scope).fallback}
                  </span>
                </li>
              ))}
            </ul>
          ) : (
            <p
              id={listId}
              role="status"
              className="px-2 py-1.5 text-sm text-muted-foreground"
              data-testid="settings-search-empty"
            >
              No setting named that.
            </p>
          )}
        </div>
      ) : null}
    </div>
  );
}
