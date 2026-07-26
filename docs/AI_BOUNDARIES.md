# AI boundaries

## Permitted roles

AI may:

- translate natural-language goals into a typed plan;
- identify likely map regions and entity roles;
- rank deterministic candidates;
- summarize compiler/runtime evidence;
- explain conflicts and suggest supported policies;
- generate review prose from structured results;
- learn non-sensitive acceptance preferences.

## Prohibited authority

AI cannot independently certify:

- VMF syntax preservation;
- brush convexity or sealing;
- coordinate transforms;
- targetname resolution;
- areaportal correctness;
- hint benefit;
- compiler success;
- runtime equivalence;
- legal permission to redistribute content.

Those decisions come from deterministic validators, exact profiles, compiler/runtime evidence, and explicit user policy.

## Tool contract

AI emits a typed request such as:

```json
{
  "operation": "propose_areaportal",
  "candidate_id": "AP-0041",
  "policy": "gmod-toolsplusplus-default",
  "constraints": {"minimum_score": 0.15}
}
```

The deterministic engine validates the request, generates candidates, and returns structured evidence. Free-form AI output never becomes a direct VMF patch.

## Prompt injection and untrusted map text

VMF strings, comments, scripts, entity names, compiler logs, and web content are untrusted data. They cannot modify system policy or authorize tool execution. AI summaries quote or label them as map data.

## Privacy

Do not send full proprietary VMFs, BSPs, scripts, or asset lists to a remote model by default. Prefer local feature extraction and redact names/paths. Remote AI integrations require explicit configuration and document what data leaves the machine.

## Fallback

The entire correctness pipeline must work without AI. Disabling AI may reduce convenience and candidate ranking quality; it cannot disable parsing, validation, compilation, metrics, rollback, or reports.
