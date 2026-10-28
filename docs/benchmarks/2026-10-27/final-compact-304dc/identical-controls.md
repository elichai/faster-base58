# All identical-content control cells

Both labels install the same wide artifact into the same global inode. Every pair is retained. Median compact-label/wide-label ratios range 0.993936–1.018033; individual pairs range 0.812408–1.114856. These are observed controls, not a guarantee of sub-percent stability.

| CPU | Policy | Observer backend | Direction | Payload | Alpha | Bravo | Median ratio | All five ratios |
|---|---|---|---|---:|---:|---:|---:|---|
| generic | cache | auto | decode | 0 B | 1.816 ns | 1.816 ns | 1.000006 | 1.000652, 1.001600, 0.999049, 1.000006, 0.999799 |
| generic | cache | auto | decode | 8 B | 12.995 ns | 12.996 ns | 1.000072 | 1.000072, 1.000450, 0.999705, 0.999820, 1.000689 |
| generic | cache | auto | decode | 23 B | 22.324 ns | 22.334 ns | 1.000176 | 1.000432, 1.000176, 0.943176, 0.998658, 1.002222 |
| generic | cache | auto | decode | 4 KiB | 27.287 µs | 27.287 µs | 0.999160 | 0.998241, 1.013669, 0.998731, 1.001381, 0.999160 |
| generic | cache | auto | decode | 4 MiB | 159.699 ms | 160.692 ms | 1.005647 | 1.005647, 1.005295, 0.997282, 1.007289, 1.028436 |
| generic | cache | auto | encode | 0 B | 2.551 ns | 2.551 ns | 1.000141 | 1.000698, 1.000141, 1.000073, 0.999459, 1.000252 |
| generic | cache | auto | encode | 8 B | 28.597 ns | 28.930 ns | 1.006703 | 0.999573, 0.999905, 1.012120, 1.006703, 1.017116 |
| generic | cache | auto | encode | 23 B | 46.725 ns | 46.903 ns | 1.003818 | 1.000017, 1.045049, 1.003818, 1.008574, 1.001131 |
| generic | cache | auto | encode | 4 KiB | 40.851 µs | 40.788 µs | 1.002317 | 0.997721, 1.002317, 0.996149, 1.004588, 1.022914 |
| generic | cache | auto | encode | 4 MiB | 322.717 ms | 322.935 ms | 0.997815 | 0.991505, 0.999585, 0.997815, 1.001396, 0.995980 |
| generic | cache | empty | decode | 0 B | 0.726 ns | 0.727 ns | 1.000356 | 0.999952, 1.000713, 1.000356, 1.000291, 1.001398 |
| generic | cache | empty | decode | 23 B | 0.726 ns | 0.726 ns | 1.000411 | 1.000411, 1.001116, 1.000072, 1.000177, 1.000444 |
| generic | cache | empty | encode | 0 B | 0.847 ns | 0.847 ns | 0.999879 | 0.999918, 0.999574, 0.999918, 0.999879, 0.999294 |
| generic | cache | empty | encode | 23 B | 0.847 ns | 0.847 ns | 0.999987 | 1.000703, 1.000592, 0.999987, 0.999740, 0.999432 |
| generic | default | auto | decode | 0 B | 2.907 ns | 2.908 ns | 1.000097 | 1.000097, 1.000422, 0.999201, 0.999913, 1.000268 |
| generic | default | auto | decode | 8 B | 11.599 ns | 11.617 ns | 1.000375 | 0.920969, 1.000375, 1.019602, 1.114856, 1.000306 |
| generic | default | auto | decode | 23 B | 21.737 ns | 21.748 ns | 1.000928 | 0.998637, 0.999673, 1.000928, 1.089899, 1.001205 |
| generic | default | auto | decode | 4 KiB | 33.151 µs | 33.158 µs | 1.000204 | 0.997812, 1.002931, 1.023503, 1.000204, 0.999958 |
| generic | default | auto | decode | 4 MiB | 166.543 ms | 164.708 ms | 0.997459 | 0.986172, 0.974684, 0.999463, 0.997459, 0.998738 |
| generic | default | auto | encode | 0 B | 2.490 ns | 2.487 ns | 0.998363 | 0.998363, 0.998161, 0.997312, 1.002275, 1.001945 |
| generic | default | auto | encode | 8 B | 28.526 ns | 28.552 ns | 1.001008 | 1.001008, 0.960540, 1.001228, 0.999765, 1.023702 |
| generic | default | auto | encode | 23 B | 53.923 ns | 53.572 ns | 0.998877 | 1.005457, 1.001513, 0.998877, 0.977101, 0.993130 |
| generic | default | auto | encode | 4 KiB | 53.799 µs | 53.762 µs | 0.998279 | 0.997057, 0.995017, 1.002099, 0.998279, 1.001060 |
| generic | default | auto | encode | 4 MiB | 350.853 ms | 349.917 ms | 1.002296 | 0.987716, 1.002651, 1.008176, 0.988747, 1.002296 |
| generic | default | empty | decode | 0 B | 0.726 ns | 0.726 ns | 1.000481 | 1.000513, 1.000233, 1.001014, 1.000244, 1.000481 |
| generic | default | empty | decode | 23 B | 0.726 ns | 0.726 ns | 1.000093 | 1.000182, 1.000066, 0.999926, 1.000093, 1.000559 |
| generic | default | empty | encode | 0 B | 0.848 ns | 0.848 ns | 0.999975 | 1.000017, 0.999359, 0.999975, 1.000018, 0.999895 |
| generic | default | empty | encode | 23 B | 0.848 ns | 0.847 ns | 0.999939 | 0.999081, 0.999427, 0.999939, 1.000418, 1.000258 |
| native | cache | auto | decode | 0 B | 1.453 ns | 1.453 ns | 1.000044 | 0.998743, 0.999873, 1.000282, 1.000193, 1.000044 |
| native | cache | auto | decode | 8 B | 12.809 ns | 12.811 ns | 1.000379 | 0.999641, 1.000379, 0.999216, 1.100356, 1.000501 |
| native | cache | auto | decode | 23 B | 25.449 ns | 25.294 ns | 0.993936 | 0.812408, 0.993936, 0.960605, 1.007523, 0.999190 |
| native | cache | auto | decode | 4 KiB | 21.947 µs | 21.949 µs | 1.000073 | 1.003440, 1.000138, 1.000073, 0.993246, 0.996231 |
| native | cache | auto | decode | 4 MiB | 154.323 ms | 153.468 ms | 0.993972 | 0.993972, 1.001048, 0.993281, 0.992168, 0.994322 |
| native | cache | auto | encode | 0 B | 2.429 ns | 2.426 ns | 0.999431 | 1.003428, 1.001068, 0.998141, 0.999431, 0.990356 |
| native | cache | auto | encode | 8 B | 28.531 ns | 29.053 ns | 1.018033 | 1.018033, 0.999588, 1.029133, 1.018058, 1.001031 |
| native | cache | auto | encode | 23 B | 45.333 ns | 45.192 ns | 0.995026 | 0.999509, 0.979489, 1.000756, 0.995026, 0.994879 |
| native | cache | auto | encode | 4 KiB | 38.705 µs | 38.775 µs | 0.998475 | 1.002934, 0.997901, 1.005189, 0.998475, 0.996769 |
| native | cache | auto | encode | 4 MiB | 312.413 ms | 311.542 ms | 0.997213 | 0.993301, 0.989051, 1.005639, 1.002430, 0.997213 |
| native | cache | empty | decode | 0 B | 0.726 ns | 0.726 ns | 1.000013 | 1.000013, 1.000246, 1.000701, 0.999497, 0.999683 |
| native | cache | empty | decode | 23 B | 0.726 ns | 0.726 ns | 1.000067 | 1.000085, 1.000213, 1.000067, 0.999887, 0.999761 |
| native | cache | empty | encode | 0 B | 0.848 ns | 0.848 ns | 1.000181 | 1.000885, 1.000181, 0.999117, 1.000608, 0.999860 |
| native | cache | empty | encode | 23 B | 0.847 ns | 0.847 ns | 1.000226 | 1.000226, 1.000238, 1.000201, 1.000708, 0.999334 |
| native | default | auto | decode | 0 B | 2.914 ns | 2.909 ns | 0.998443 | 0.997485, 1.002013, 0.998443, 1.001182, 0.995973 |
| native | default | auto | decode | 8 B | 12.850 ns | 12.842 ns | 0.999424 | 0.982885, 1.076841, 0.998676, 1.000284, 0.999424 |
| native | default | auto | decode | 23 B | 33.989 ns | 34.011 ns | 0.999141 | 0.999138, 1.000217, 1.002273, 0.999141, 0.978675 |
| native | default | auto | decode | 4 KiB | 21.896 µs | 21.908 µs | 0.998376 | 1.001772, 0.993660, 1.004866, 0.998376, 0.989276 |
| native | default | auto | decode | 4 MiB | 153.286 ms | 153.291 ms | 0.998969 | 0.995075, 0.998969, 0.983103, 1.000097, 1.017586 |
| native | default | auto | encode | 0 B | 2.670 ns | 2.668 ns | 0.999283 | 1.030173, 1.000001, 0.998098, 0.999283, 0.971226 |
| native | default | auto | encode | 8 B | 28.832 ns | 28.836 ns | 0.994701 | 0.994004, 0.994701, 0.991565, 0.997430, 1.013776 |
| native | default | auto | encode | 23 B | 53.957 ns | 53.925 ns | 0.999404 | 0.999404, 1.022678, 0.999597, 0.998821, 0.966685 |
| native | default | auto | encode | 4 KiB | 51.168 µs | 51.312 µs | 1.002823 | 1.010065, 1.012090, 0.997665, 1.002703, 1.002823 |
| native | default | auto | encode | 4 MiB | 337.527 ms | 337.787 ms | 0.999679 | 1.002809, 0.999679, 0.998829, 0.988514, 1.006238 |
| native | default | empty | decode | 0 B | 0.726 ns | 0.727 ns | 1.000226 | 0.999567, 1.001263, 1.000092, 1.000226, 1.001136 |
| native | default | empty | decode | 23 B | 0.726 ns | 0.727 ns | 1.000186 | 0.998118, 1.001646, 1.000186, 1.000688, 0.999725 |
| native | default | empty | encode | 0 B | 0.847 ns | 0.847 ns | 0.999905 | 1.000119, 0.999442, 0.999905, 1.000663, 0.999806 |
| native | default | empty | encode | 23 B | 0.848 ns | 0.847 ns | 1.000032 | 1.000388, 1.001300, 0.998015, 0.998954, 1.000032 |
