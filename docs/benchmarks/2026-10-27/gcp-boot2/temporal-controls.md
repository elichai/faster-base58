# Temporal Auto controls

The same Auto route is remeasured in later NTT/development units using the same frozen worker. Ratios below divide those nine-batch warm medians by the earlier public-unit Auto median. All 464 cells remain in the CSV. These ranges expose time/context variation; they are not algorithm speedups or confidence bounds.

| CPU | Policy | Later unit | Minimum ratio: payload/direction | Maximum ratio: payload/direction |
|---|---|---|---:|---:|
| gcp-generic | default | ntt | 0.948261: 21 B decode | 1.024478: 31 B decode |
| gcp-generic | default | development | 0.947500: 21 B decode | 1.038220: 23 B decode |
| gcp-generic | cache | ntt | 0.961122: 8 B decode | 1.021802: 8 B encode |
| gcp-generic | cache | development | 0.954624: 8 B decode | 1.023201: 21 B encode |
| gcp-native | default | ntt | 0.895129: 32 B encode | 1.027465: 128 B decode |
| gcp-native | default | development | 0.946807: 32 B decode | 1.034387: 1 B encode |
| gcp-native | cache | ntt | 0.954053: 22 B decode | 1.037641: 8 B decode |
| gcp-native | cache | development | 0.957034: 23 B decode | 1.068490: 63 B decode |
