# Partition Number

test how `partition_num` effects decoding speed given a large enough decoding problem.

## Generating syndrome patterns

```sh
cargo run --release -- benchmark 15 -n 100000 0.005 --code-type phenomenological-planar-code-parallel --code-config '{"code_count":10}' --primal-dual-type error-pattern-logger --verifier none --primal-dual-config '{"filename":"benchmark/partition_num/15-100000-0.005-phenomenological-planar.syndromes"}'
```
