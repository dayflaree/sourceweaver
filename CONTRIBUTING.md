# Contributing

## Before changing code

1. Read [Automation contract](docs/AUTOMATION_CONTRACT.md).
2. Read the matching project skill under [`skills/`](skills/README.md).
3. Update the support envelope, invariants, and fixtures with any new behavior.
4. Use synthetic, redistributable test data.

## Development

```bash
python -m venv .venv
# activate the environment
python -m pip install -e ".[dev]"
ruff check .
mypy
pytest --cov
```

## Pull requests

Include:

- exact behavior changed;
- evidence and design decision;
- validation actually run;
- compiler/runtime fingerprints for integration results;
- any remaining unsupported condition.

A compile screenshot or anecdotal FPS result is insufficient evidence.

## Research changes

Add or update entries in `docs/RESEARCH_LEDGER.md` and `docs/SOURCE_INDEX.md`. Distinguish first-party facts, source-code observations, local executable observations, engineering conclusions, and planned experiments.

## Content policy

Never commit copyrighted game maps/assets or redistributable compiler binaries. Provide a synthetic fixture.
