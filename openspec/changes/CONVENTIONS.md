# Conventions for New Deterministic Checks

This document establishes shared conventions across all new check proposals, ensuring consistency in naming, reporting, exclusion rules, and pattern-registry design.

## Rule ID Naming

All SARIF rule IDs follow the pattern `pretender/<metric-name>` where `<metric-name>` uses kebab-case:

| Rule ID | Metric | Notes |
|---|---|---|
| `pretender/mock-overuse` | Mock reference count per file | |
| `pretender/mutable-ratio` | Mutable binding ratio per file | |
| `pretender/lazy-cluster` | Duplicate test assertion cluster | |
| `pretender/bool-cluster` | Boolean parameter cluster per function | |
| `pretender/primitive-param` | Primitive type used in place of domain type | |
| `pretender/void-mutator` | Void-return mutation methods per file | |
| `pretender/unwrap-density` | Unwrap/except/catch calls per code unit | |
| `pretender/inheritance-depth` | Class inheritance chain depth | |
| `pretender/efferent-coupling` | Efferent coupling (Ce) per module | Well-known CE abbreviation accepted in output |
| `pretender/afferent-coupling` | Afferent coupling (Ca) per module | Well-known CA abbreviation accepted in output |
| `pretender/coupling-between-objects` | Coupling between objects (CBO) per module | Well-known CBO abbreviation accepted in output |
| `pretender/lcom-hs` | Lack of cohesion (LCOM-HS) per class | Standard abbreviation kept |
| `pretender/import-cycle` | Cycle in the module dependency graph | |

## Threshold Field Naming

All threshold config fields use `snake_case`:

```
[thresholds]
mock_count_max = 5               # max mock references per test file
mut_ratio_max = 0.3               # max mutable-to-total-binding ratio
lazy_cluster_min = 3              # min cluster size to flag
bool_cluster_max = 3              # max bool params per function
primitive_param_check = false     # enable primitive domain param detection
void_mutators_max = 3             # max void-mutator methods per file
unwrap_max = 5                    # max unwrap/except per code unit
inheritance_depth_max = 2         # max inheritance depth
ce_max = 9                        # max efferent coupling
ca_max = 9                        # max afferent coupling
cbo_max = 9                       # max coupling between objects
lcom_hs_max = 30                  # max LCOM-HS percentage
cycle_detection = false           # enable cycle detection

[thresholds.test]
mock_count_max = 5
unwrap_max = 5
```

## Generated and Vendor Exclusion

All new checks SHALL exclude files with role `generated` or `vendor` by default. This SHALL be documented in each spec as an explicit exclusion scenario. The exclusion follows the existing role resolution order — if a file is assigned `generated` or `vendor`, no new check findings are emitted for it.

## Shared Pattern Registry

Per-language detection patterns (mock libraries, binding syntax, exception keywords, etc.) SHALL be registered in a shared `language_patterns` module rather than duplicated across individual check modules. This prevents drift and makes adding new languages or patterns a single point of change.

```
pretender/src/
  language_patterns.rs    # shared registry: MockLibrary, BindingStyle, ExceptionPattern, etc.
  mock_detector.rs        # uses language_patterns::MOCK_LIBRARIES
  mutability_metrics.rs   # uses language_patterns::BINDING_STYLES
  error_metrics.rs        # uses language_patterns::EXCEPTION_PATTERNS
  ...
```

## User-Extensible Patterns

Any check with a finite list of per-language patterns (mock libraries, domain-primitive name heuristics) SHALL provide a `[patterns]` config section so users can add custom patterns without modifying the binary:

```
[patterns.mock]
extra = ["my_lib::Mock", "custom_mock"]

[patterns.primitive_params]
extra = ["token", "secret"]
```

## Implementation Sequencing

The proposals have the following dependency graph:

```
Foundation layer (build first):
  └─ add-coupling-and-cohesion-metrics  # requires import resolution — foundational
  └─ add-primitive-obsession-check      # requires @param.type captures — touches all adapters

Independent checks (any order, after foundation):
  └─ add-mock-overuse-check
  └─ add-mutable-state-check            # shares mutability_metrics.rs with void-mutator
  └─ add-void-mutator-check             # implement same time as mutable-state
  └─ add-unwrap-density-check
  └─ add-lazy-test-cluster-check
  └─ add-inheritance-depth-check
```

The two mutability proposals (`add-mutable-state-check` + `add-void-mutator-check`) share `mutability_metrics.rs` — they should be implemented together or in immediate succession.