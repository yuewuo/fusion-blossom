# Serial Speed

test how fast can my serial version run.

## About my machine

See [partition_num](../partition_num/README.md).

## Generating syndrome patterns

```sh
# d = 3
cargo run --release -- benchmark 3 -n 1000 0.005 --code-type phenomenological-planar-code-parallel --code-config '{"code_count":10}' --primal-dual-type error-pattern-logger --verifier none --primal-dual-config '{"filename":"benchmark/serial_speed/3-1000-0.005-phenomenological-planar.syndromes"}'
# d = 5
cargo run --release -- benchmark 5 -n 1000 0.005 --code-type phenomenological-planar-code-parallel --code-config '{"code_count":10}' --primal-dual-type error-pattern-logger --verifier none --primal-dual-config '{"filename":"benchmark/serial_speed/5-1000-0.005-phenomenological-planar.syndromes"}'
# d = 7
cargo run --release -- benchmark 7 -n 1000 0.005 --code-type phenomenological-planar-code-parallel --code-config '{"code_count":10}' --primal-dual-type error-pattern-logger --verifier none --primal-dual-config '{"filename":"benchmark/serial_speed/7-1000-0.005-phenomenological-planar.syndromes"}'
# d = 9
cargo run --release -- benchmark 9 -n 1000 0.005 --code-type phenomenological-planar-code-parallel --code-config '{"code_count":10}' --primal-dual-type error-pattern-logger --verifier none --primal-dual-config '{"filename":"benchmark/serial_speed/9-1000-0.005-phenomenological-planar.syndromes"}'
# d = 15
cargo run --release -- benchmark 15 -n 1000 0.005 --code-type phenomenological-planar-code-parallel --code-config '{"code_count":10}' --primal-dual-type error-pattern-logger --verifier none --primal-dual-config '{"filename":"benchmark/serial_speed/15-1000-0.005-phenomenological-planar.syndromes"}'
# d = 21
cargo run --release -- benchmark 21 -n 1000 0.005 --code-type phenomenological-planar-code-parallel --code-config '{"code_count":10}' --primal-dual-type error-pattern-logger --verifier none --primal-dual-config '{"filename":"benchmark/serial_speed/21-1000-0.005-phenomenological-planar.syndromes"}'
# d = 27
cargo run --release -- benchmark 27 -n 1000 0.005 --code-type phenomenological-planar-code-parallel --code-config '{"code_count":10}' --primal-dual-type error-pattern-logger --verifier none --primal-dual-config '{"filename":"benchmark/serial_speed/27-1000-0.005-phenomenological-planar.syndromes"}'
# d = 45
cargo run --release -- benchmark 45 -n 1000 0.005 --code-type phenomenological-planar-code-parallel --code-config '{"code_count":10}' --primal-dual-type error-pattern-logger --verifier none --primal-dual-config '{"filename":"benchmark/serial_speed/45-1000-0.005-phenomenological-planar.syndromes"}'
# d = 63
cargo run --release -- benchmark 63 -n 1000 0.005 --code-type phenomenological-planar-code-parallel --code-config '{"code_count":10}' --primal-dual-type error-pattern-logger --verifier none --primal-dual-config '{"filename":"benchmark/serial_speed/63-1000-0.005-phenomenological-planar.syndromes"}'
# d = 81
cargo run --release -- benchmark 81 -n 1000 0.005 --code-type phenomenological-planar-code-parallel --code-config '{"code_count":10}' --primal-dual-type error-pattern-logger --verifier none --primal-dual-config '{"filename":"benchmark/serial_speed/81-1000-0.005-phenomenological-planar.syndromes"}'
```

## Evaluating decoding time under various partition number

```sh
cargo run --release -- benchmark 3 -n 1000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/serial_speed/3-1000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type serial --verifier none --benchmark-profiler-output benchmark/serial_speed/3-1000-0.005-phenomenological-planar.profile
cargo run --release -- benchmark 5 -n 1000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/serial_speed/5-1000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type serial --verifier none --benchmark-profiler-output benchmark/serial_speed/5-1000-0.005-phenomenological-planar.profile
cargo run --release -- benchmark 7 -n 1000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/serial_speed/7-1000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type serial --verifier none --benchmark-profiler-output benchmark/serial_speed/7-1000-0.005-phenomenological-planar.profile
cargo run --release -- benchmark 9 -n 1000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/serial_speed/9-1000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type serial --verifier none --benchmark-profiler-output benchmark/serial_speed/9-1000-0.005-phenomenological-planar.profile
cargo run --release -- benchmark 15 -n 1000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/serial_speed/15-1000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type serial --verifier none --benchmark-profiler-output benchmark/serial_speed/15-1000-0.005-phenomenological-planar.profile
cargo run --release -- benchmark 21 -n 1000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/serial_speed/21-1000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type serial --verifier none --benchmark-profiler-output benchmark/serial_speed/21-1000-0.005-phenomenological-planar.profile
cargo run --release -- benchmark 27 -n 1000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/serial_speed/27-1000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type serial --verifier none --benchmark-profiler-output benchmark/serial_speed/27-1000-0.005-phenomenological-planar.profile
cargo run --release -- benchmark 45 -n 1000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/serial_speed/45-1000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type serial --verifier none --benchmark-profiler-output benchmark/serial_speed/45-1000-0.005-phenomenological-planar.profile
cargo run --release -- benchmark 63 -n 1000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/serial_speed/63-1000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type serial --verifier none --benchmark-profiler-output benchmark/serial_speed/63-1000-0.005-phenomenological-planar.profile
cargo run --release -- benchmark 81 -n 1000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/serial_speed/81-1000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type serial --verifier none --benchmark-profiler-output benchmark/serial_speed/81-1000-0.005-phenomenological-planar.profile
```
