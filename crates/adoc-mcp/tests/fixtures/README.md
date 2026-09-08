# Migration export conformance fixture

`migration-export-native.json` retains unmodified JSON values emitted by the Cloud
T5 actual-runtime and authorized-human integration run on 2026-09-08. It contains
one complete successful export group and the syntax-invalid and invalid-UTF8
qualification source-evidence groups. The duplicate successful export group was
omitted. The fixture contains generated test identities, not credentials.

These are native export JSON values, re-indented for this fixture. They do not
claim original retained wire-byte encoding. The tests validate published shape;
Cloud integration separately proves authorization, causal joins and roundtrip
behavior. No fixture record grants activation authority.

## Migration lifecycle fixture

`migration-lifecycle-native.json` is a byte-identical copy of the E7.2.T1 real
PostgREST conformance capture from 2026-09-08. It retains six native result
objects (including original base64 receipt bytes), their decoded receipt JSON,
five actual command inputs and seven exported native facts. The real pinned
runtime and signed-in human initialization case reached `catching_up`; it did
not perform cutover. Runtime SHA256:
`8c9093250e761f7d772a95c64ce2b4825388a5b6a3ccf3207e5c6c8d0308eda5`.

All six receipt byte digests, LF endings and decoded JSON correspondence were
verified when copying this fixture. The registered shape tests retain the byte
strings unchanged and check the captured ordering and references. The Cloud
suite independently tests native authorization, concurrency and causal
admission. No handmade output or reconstructed original prepare receipt is
included; `preparation_validation` is native metadata projection only.
