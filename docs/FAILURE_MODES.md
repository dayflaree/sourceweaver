# Failure modes and automatic responses

| Failure | Detection | Automatic response |
|---|---|---|
| VMF parse error | CST parser | Block mutation; report exact span |
| Round-trip mismatch | byte comparison | Block all transforms |
| Unknown nested editor data on changed object | semantic/CST reconciliation | Preserve object unchanged or block candidate |
| Invalid/non-convex brush | geometry reconstruction | Mark non-transformable; continue analysis |
| Numerical ambiguity | robust predicate fallback | Increase precision; block if unresolved |
| ID collision | allocation check | Deterministically reallocate |
| Ambiguous targetname | reference graph | Block namespace transaction |
| Opaque script dependency | script risk scan | Require a supported rewrite rule or block |
| Alignment mismatch | seam scoring | Reject hypothesis; do not search arbitrary transforms by default |
| Conflicting seam geometry | overlap classifier | Keep both only if compiler-safe; otherwise block |
| World bounds/budget risk | capacity planner | Reject before compile |
| Compiler missing | doctor/profile check | Analysis-only mode |
| Compiler hash changed | fingerprint comparison | Quarantine and run qualification |
| Compiler timeout/hang | watchdog | Kill process tree; retain artifacts; reject |
| World leak | VBSP log/line file | Reject candidate and attach leak evidence |
| Areaportal leak or area mismatch | VBSP/BSP inspection | Reject portal candidate |
| Limit overflow | log/lump checks | Reject; suggest partition/alternate strategy |
| New warning | normalized log diff | Reject unless policy explicitly permits the code |
| Missing resource | compile/runtime logs | Resolve from legal search paths or reject |
| Runtime crash/hang | process watchdog | Reject and retain crash/log fingerprints |
| Behavior assertion failure | scenario result | Reject and report first causal divergence |
| Inconclusive mandatory test | insufficient observability | Block automatic acceptance |
| Performance noise | repeated statistics | Do not accept as optimization |
| External file changed mid-run | hash recheck | Abort and restart only by explicit new run |
| Insufficient disk space | preflight/monitor | Abort cleanly, keep baseline intact |
| Unauthorized content input/output | provenance policy | Refuse packaging/distribution |

## Recovery

All generated work is content-addressed and disposable. Recovery never edits source files. A failed run can resume completed immutable stages only when all input/profile/tool hashes still match.
