#ifndef ENERGY_BENCH_H
#define ENERGY_BENCH_H

#include <stdint.h>

typedef struct BenchStart BenchStart;

typedef struct BenchResult {
  char *const *keys;
  const float *values;
  uintptr_t len;
} BenchResult;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

struct BenchStart *energy_bench_init(uintptr_t idle_duration_seconds);

void energy_bench_start(struct BenchStart *start);

struct BenchResult *energy_bench_stop(struct BenchStart *start);

void energy_bench_free(struct BenchStart *res);

void energy_bench_res_free(struct BenchResult *res);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* ENERGY_BENCH_H */
