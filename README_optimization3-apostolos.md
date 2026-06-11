# optimization3-apostolos Branch README

## Purpose

This branch applies the green-mode approach described in Apostolos' report, but
keeps the existing Rust benchmark script unchanged so the results can be
compared directly with previous runs.

In the report, green mode is opt-in through `--green`. In this branch, green
mode is enabled by default so that the benchmark command:

```text
bundle exec ./bin/image_optim -r runs/bench
```

measures the green profile without changing
`energy-benchmarking/example/src/image_optim.rs`.

The original profile can still be restored with:

```sh
./bin/image_optim --no-green ...
```

or:

```ruby
ImageOptim.new(green: false)
```

## Implemented Changes

### Green Worker Defaults

The green profile changes worker defaults when the user has not explicitly
configured those workers:

```ruby
advpng: false
pngout: false
pngcrush: false
optipng: {level: 2}
oxipng: {level: 2}
gifsicle: {interlace: false}
jpegtran: {jpegrescan: false}
```

This avoids expensive late compression work while keeping lower-cost useful
passes available.

### Early Exit

The optimizer stops launching additional workers for an image once the current
best result is reduced enough compared with the original.

Default threshold:

```text
green_threshold = 0.50
```

That means later workers are skipped once the current output is at least 50%
smaller than the input.

### Timeout

Green mode sets a default per-image timeout:

```text
timeout = 30 seconds
```

Explicit timeout configuration still wins.

### Thread Cap

Green mode caps the default thread count to:

```text
2 threads
```

Explicit `threads` configuration still wins.

### CLI Controls

The following options were added:

```sh
--green
--no-green
--green-threshold N
```

Because green mode is already enabled by default in this branch, `--no-green`
is the useful control for restoring the original profile.

### Cache Safety

The cache key now includes:

```text
green
green_threshold
```

This prevents reusing cached outputs generated under a different green
configuration.

## Benchmark Script

The Rust benchmark script was intentionally left unchanged:

```text
energy-benchmarking/example/src/image_optim.rs
```

This keeps the benchmark structure, datasets, ablation labels, setup/cleanup,
and size-reduction CSV behavior directly comparable with earlier runs.

## Validation

Run these checks before measuring:

```sh
ruby -c lib/image_optim.rb
ruby -c lib/image_optim/config.rb
ruby -c lib/image_optim/cache.rb
ruby -c lib/image_optim/runner/option_parser.rb
bundle exec rspec \
  spec/image_optim/config_spec.rb \
  spec/image_optim/runner/option_parser_spec.rb \
  spec/image_optim_spec.rb:147
```

