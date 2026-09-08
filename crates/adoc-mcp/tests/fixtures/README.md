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
