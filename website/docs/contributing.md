---
title: Contributing
description: "How to contribute to agnix -- report bugs, request rules, improve docs, or write code."
---

# Contributing

Contributions are welcome and appreciated.

## Found something off?

agnix validates against 139 rules, but the agent config ecosystem moves fast. If a rule is wrong, missing, or too noisy, we want to know.

- [Report a bug](https://github.com/avifenesh/agnix/issues/new)
- [Request a rule](https://github.com/avifenesh/agnix/issues/new)

Your real-world configs are the best test suite we could ask for.

## Contribute code

Good first issues are labeled and ready:
[good first issues](https://github.com/avifenesh/agnix/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22)

Adding a new rule is one of the best ways to get started. Each rule is a self-contained unit with clear inputs, outputs, and test patterns. Find a similar existing rule to use as your template.

## Improve docs

This documentation site is in `website/`. To run locally:

```bash
npm --prefix website ci
npm --prefix website run generate:rules
npm --prefix website start
```

## Where canonical content lives

Long-form source-of-truth docs remain in the repository:

- `README.md`
- `SPEC.md`
- `knowledge-base/`

This website assembles and links that content for navigation and search.

## References

- [CONTRIBUTING.md](https://github.com/avifenesh/agnix/blob/main/CONTRIBUTING.md) -- full contribution guidelines
- [SECURITY.md](https://github.com/avifenesh/agnix/blob/main/SECURITY.md) -- security policy
