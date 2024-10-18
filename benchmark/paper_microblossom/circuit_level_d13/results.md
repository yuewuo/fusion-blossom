
# Parity Blossom


note: evaluation in micro blossom shows 14.2us decoding latency, probably because micro blossom will process
the graph and remove duplicated virtual vertices, etc. fusion blossom does not natively have this.

17.6us in total using naive graph

```
noisy_measurements: 12
    average_decoding_time: 1.7573666515329313e-05
    average_decoding_time_per_round: 1.351820501179178e-06
    average_decoding_time_per_defect: 1.1764147328157196e-06
    average_defect_per_measurement: 1.1491019820396409
    decoding_time_relative_dev: 0.46450437187370397
```

# Sparse Blossom

6.6us in total

```
1000008
initializer loaded
initializer created
matching initialized
decoding time: 6.5828663330030395
average decoding latency: 6.582945e-06
```
