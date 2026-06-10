# optimization1-ale Branch README

## Purpose

This branch introduces energy-oriented improvements to `image_optim`.
The goal is to reduce unnecessary CPU scheduling, process work, and system
priority pressure while keeping the optimizer behavior close to the original
project and preserving the energy benchmark setup.

The benchmark file at:

```text
energy-benchmarking/example/src/image_optim.rs
```

was intentionally left unchanged. In particular, the ablation study structure,
dataset names, benchmark labels, Linux repository path, and benchmark working
directories were preserved so that measurements remain comparable when run on
Linux.

## High-Level Summary

The branch makes three main runtime changes:

1. `threads` is now treated as a maximum, and small image batches use fewer
   worker threads automatically.
2. The optimizer can stop late-stage workers when they provide only a very
   small additional size reduction.
3. The default process nice level becomes more energy-aware on battery power.

The branch also updates the CLI documentation, project README, and tests to
describe and validate the new behavior.

## Files Changed

### `lib/image_optim.rb`

This is the main runtime change.

The branch adds adaptive threading constants:

```ruby
SMALL_IMAGE_SIZE = 100 * 1024
MEDIUM_IMAGE_SIZE = 1024 * 1024
SMALL_IMAGE_THREADS = 2
MEDIUM_IMAGE_THREADS = 4
MIN_WORKER_GAIN_RATIO = 0.01
```

These constants define the energy policy used by the optimizer:

- Images below 100 KB are considered small.
- Images below 1 MB are considered medium.
- Late-stage worker gains below 1% of the original file size are considered
  too small to justify continuing through the rest of the worker chain.

### `lib/image_optim/config.rb`

This branch changes the default nice level behavior.

Before this branch:

- `nice: nil` or `nice: true` always resulted in nice level `10`.

After this branch:

- `nice: nil` or `nice: true` gives nice level `10` on normal power.
- `nice: nil` or `nice: true` gives nice level `15` when the system appears to
  be running on battery power.
- `nice: false` still gives nice level `0`.
- An explicit numeric nice value is still respected.

This keeps existing user configuration behavior intact while making the default
more conservative on battery-powered machines.

### `lib/image_optim/runner/option_parser.rb`

The CLI help text was updated so that `--threads` is described as the maximum
number of threads, not a guaranteed number of active threads.

The `--nice` help text was also updated to mention the battery-aware default.

### `README.markdown`

The public project README was updated to document the new meaning of `:threads`
and the battery-aware default for `:nice`.

### `spec/image_optim_spec.rb`

New tests were added for:

- Stopping late workers after a minimal additional gain.
- Capping tiny image batches to two threads.
- Capping medium image batches to four threads.
- Keeping the configured maximum thread count for large image batches.

### `spec/image_optim/config_spec.rb`

New tests were added for the battery-aware nice default.

## Detailed Change 1: Adaptive Threading

### Previous Behavior

Previously, `apply_threading` used a simple rule:

```ruby
if threads > 1
  enum.in_threads(threads)
else
  enum
end
```

That means the configured thread count was always used whenever it was greater
than one.

For example, if the machine had 8 logical processors and `threads` defaulted to
8, then even a batch of tiny images could be processed with 8 concurrent worker
threads.

### New Behavior

This branch keeps `threads` as the user-configured maximum, but it may use fewer
threads for small or medium image batches:

```text
Average image size < 100 KB  -> max 2 threads
Average image size < 1 MB    -> max 4 threads
Average image size >= 1 MB   -> configured max threads
```

For example:

- If `threads` is `8` and the average file is `10 KB`, the optimizer uses `2`
  threads.
- If `threads` is `8` and the average file is `500 KB`, the optimizer uses `4`
  threads.
- If `threads` is `8` and the average file is `2 MB`, the optimizer can still
  use `8` threads.

### Why This Is Greener

Small files often finish quickly, so using many threads can waste energy through:

- thread scheduling overhead,
- context switching,
- process startup pressure,
- short bursts of CPU activity across many cores,
- reduced locality and more system coordination work.

For small files, the overhead of using all available cores can be larger than
the benefit. By capping thread count for small batches, the optimizer should use
less CPU coordination while still maintaining useful parallelism.

### Compatibility Notes

The user-facing `threads` option still works. It now represents a ceiling rather
than a promise that exactly that many threads will be used.

Passing `threads: false` still disables parallel processing by resolving to one
thread.

## Detailed Change 2: Conservative Early Stop for Late Workers

### Previous Behavior

For a given image, `image_optim` ran every configured worker for that image
format in sequence. If an early worker already achieved most of the useful
compression, later workers would still run.

This can waste energy because several image optimization tools are CPU-heavy and
may produce only tiny additional savings after earlier tools have already done
the important work.

### New Behavior

This branch tracks the size reduction achieved by each successful worker.

After a successful late-stage worker, the optimizer compares the additional
savings against a minimum gain threshold:

```text
minimum gain = original file size * 0.01
```

In other words, if a late-stage worker saves less than 1% of the original file
size, the optimizer stops running the remaining late-stage workers.

The stop is deliberately conservative:

- It only happens after a worker succeeds.
- It only happens for workers with `run_order > 0`.
- Earlier workers still get a chance to run.
- Failed workers do not trigger early stopping.
- Timeout behavior remains unchanged.

### Why This Is Greener

Image optimization often has diminishing returns. The first useful worker may
remove most redundant data, while later workers spend significant CPU time
trying to save a few more bytes.

Stopping after very small late-stage gains reduces:

- unnecessary external process execution,
- CPU time spent chasing tiny improvements,
- temporary file churn,
- energy spent on low-value work.

### Expected Trade-Off

Some images may be slightly larger than they would be if every possible worker
always ran. This is an intentional energy trade-off: the branch prioritizes
avoiding low-value work once the remaining expected gain is minimal.

## Detailed Change 3: Battery-Aware Nice Level

### Previous Behavior

The default nice level was always `10`.

This means external optimizer tools were already run at a lower priority than
normal user processes, but the default did not change based on power state.

### New Behavior

The default nice level is now:

```text
10 on normal power
15 on battery power
```

The optimizer checks battery state on:

- Linux, using `/sys/class/power_supply/BAT*/status`
- macOS, using `pmset -g batt`

Explicit configuration still wins. For example:

```yaml
nice: 20
```

or:

```ruby
ImageOptim.new(nice: 0)
```

will continue to use the value requested by the user.

### Why This Is Greener

A higher nice value means lower process priority. On battery power, this makes
the optimizer less aggressive and less likely to compete heavily with foreground
work.

This does not directly guarantee lower total energy in every situation, but it
encourages more conservative scheduling and avoids treating battery-powered
execution the same as plugged-in execution.

## Benchmark Fairness

The energy benchmark file was not changed:

```text
energy-benchmarking/example/src/image_optim.rs
```

The following benchmark properties remain the same:

- The Linux `REPO_PATH` constant is unchanged.
- The benchmark directory remains `runs/bench`.
- Dataset preparation remains outside the measured phase.
- Cleanup remains outside the measured phase.
- The measured phase still runs:

```text
bundle exec ./bin/image_optim -r runs/bench
```

- The ablation variants are unchanged:
  - `full_all_optimizers`
  - `no_svgo`
  - `no_pngout`
  - `no_optipng`
  - `no_pngcrush`
  - `no_advpng`
  - `no_pngquant`
  - `no_jpegoptim`
  - `no_jpegtran`
  - `no_gifsicle`

The GIF-specific benchmark also still keeps:

- `full_all_optimizers`
- `no_gifsicle`

### Important Measurement Note

Because this branch introduces battery-aware nice behavior, comparisons should
be run under the same power conditions.

For fair comparison between `main` and `optimization1-ale`:

- run both branches on the same machine,
- use the same installed optimizer binaries,
- use the same datasets,
- use the same power state,
- avoid switching between battery and AC power during a benchmark session,
- keep CPU governor, thermal state, and background workload as stable as
  possible.

If the goal is to isolate only the adaptive threading and early-stop changes,
run both branches while plugged in so the default nice level remains `10`.

## Size-Reduction CSV

The Rust benchmark harness now also writes a size-reduction sidecar CSV:

```text
runs/image_optim_size_reductions.csv
```

This file is generated by:

```text
energy-benchmarking/example/src/image_optim.rs
```

The CSV is useful for checking whether lower energy consumption comes with a
file-size trade-off.

Each row records one dataset and ablation variant:

```text
bench_name,dataset,variant,status,file_count,original_size_bytes,optimized_size_bytes,reduced_bytes,reduced_kb,reduction_percent,error
```

Column meanings:

- `bench_name`: energy benchmark group name.
- `dataset`: source dataset used for the run.
- `variant`: ablation variant, such as `full_all_optimizers` or `no_pngout`.
- `status`: `ok` if the size pass succeeded, `failed` otherwise.
- `file_count`: number of files in the copied benchmark input.
- `original_size_bytes`: total size before `image_optim` runs.
- `optimized_size_bytes`: total size after `image_optim` runs.
- `reduced_bytes`: byte reduction from original to optimized output.
- `reduced_kb`: byte reduction converted to KB.
- `reduction_percent`: percentage reduction relative to original size.
- `error`: failure message if the size pass failed.

The size accounting pass is intentionally run after all `EnergyBench`
measurements have finished. Then the harness performs one extra non-measured
optimization pass for each dataset and ablation variant to calculate the size
reduction and append the CSV row. This avoids adding directory traversal, CSV
writing, or extra optimizer runs to the measured energy phase, and it also
avoids heating the machine between energy benchmark variants.

## Validation Performed

The following checks were run after implementing the branch changes:

```sh
ruby -c lib/image_optim.rb
ruby -c lib/image_optim/config.rb
ruby -c lib/image_optim/runner/option_parser.rb
```

All returned:

```text
Syntax OK
```

Focused specs were also run:

```sh
bundle exec rspec \
  spec/image_optim_spec.rb:147 \
  spec/image_optim_spec.rb:165 \
  spec/image_optim/config_spec.rb \
  spec/image_optim/runner/option_parser_spec.rb
```

Result:

```text
47 examples, 0 failures
```

Whitespace validation was also run:

```sh
git diff --check
```

No whitespace errors were reported.

After adding the size-reduction CSV, the Rust benchmark target was checked with:

```sh
cargo check --manifest-path energy-benchmarking/example/Cargo.toml --bin image_optim
```

Result:

```text
Finished `dev` profile [unoptimized + debuginfo] target(s)
```

## Known Local Test Limitation

Running the broader `spec/image_optim_spec.rb` in this local environment failed
because the `svgo` binary is not installed.

The failure is unrelated to this branch's code changes. The error reported was:

```text
svgo worker: `svgo` not found
```

On a machine with all optimizer binaries installed, the full worker integration
suite should be rerun.

## Expected Impact

This branch should reduce energy consumption mainly by reducing unnecessary work
in two places:

1. Batch-level parallelism:
   - fewer threads for small and medium image batches,
   - less context switching,
   - less CPU scheduling overhead.

2. Per-image worker execution:
   - fewer late-stage external optimizer runs when additional savings become
     very small,
   - less CPU time spent on diminishing returns,
   - less temporary file churn.

The most visible differences are expected on datasets containing many small or
already-easy-to-optimize images. Large images and batches with meaningful
remaining savings should still use the configured thread capacity and continue
through the normal worker chain.

## How to Review This Branch

Useful commands:

```sh
git status --short --branch
git diff --stat main...optimization1-ale
git diff main...optimization1-ale
```

Useful focused tests:

```sh
bundle exec rspec \
  spec/image_optim_spec.rb:147 \
  spec/image_optim_spec.rb:165 \
  spec/image_optim/config_spec.rb \
  spec/image_optim/runner/option_parser_spec.rb
```

Useful benchmark command from the Rust benchmark directory:

```sh
cd energy-benchmarking/example
cargo run --bin image_optim
```

Make sure the `REPO_PATH` inside `energy-benchmarking/example/src/image_optim.rs`
matches the Linux path where the repository is located before running the
benchmark on Linux.
