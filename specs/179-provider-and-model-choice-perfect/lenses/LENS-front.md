# Lens — Front

Parent: [15-cross-ref](../15-cross-ref.md)

## WHY this lens

`ModelPickerPanel` merges two lists. The chip and the wizard then disagree
with the role editor, which already did this correctly. The front-end fix
is subtraction.

## One data flow

```text
  GET /models/eligible?scope&usage
        |
        v
  useScopedEligibleModels
        |
        v
  ModelChoiceMenu
        |
        +--> role editor (PUT llm_roles)
        +--> chip (local slot, per workspace)
        +--> dropzone vision
        +--> wizards (scope chosen by the parent)
```

No arrow from `GET /models/search` into that tree. The refresh review reads
`provider_model_cache` through the deployment provider payload.

## Component duties

`ModelChoiceMenu` is presentational: options, value, onChange, disabled,
loading, error. Parents own the mutation. The menu does not call the API.

`useDirtyGuard(dirty)` is the only navigation guard. Cards do not add their
own `beforeunload`.

## Tests

Playwright stubs eligible and search with disjoint ids. The unit test for
the menu feeds both and asserts the search id is absent. Typecheck fails
if a choice file imports `fetchLlmModels` or `useLlmModels`.
