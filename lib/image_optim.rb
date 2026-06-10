# frozen_string_literal: true

require 'image_optim/benchmark_result'
require 'image_optim/bin_resolver'
require 'image_optim/cache'
require 'image_optim/config'
require 'image_optim/errors'
require 'image_optim/handler'
require 'image_optim/image_meta'
require 'image_optim/optimized_path'
require 'image_optim/path'
require 'image_optim/table'
require 'image_optim/timer'
require 'image_optim/worker'
require 'in_threads'
require 'shellwords'

%w[
  pngcrush pngout advpng optipng pngquant oxipng
  jhead jpegoptim jpegrecompress jpegtran
  gifsicle
  svgo
].each do |worker|
  require "image_optim/worker/#{worker}"
end

# Main interface
class ImageOptim
  SMALL_IMAGE_SIZE = 100 * 1024
  MEDIUM_IMAGE_SIZE = 1024 * 1024
  SMALL_IMAGE_THREADS = 2
  MEDIUM_IMAGE_THREADS = 4
  MIN_WORKER_GAIN_RATIO = 0.01

  # Nice level
  attr_reader :nice

  # Number of threads to run with
  attr_reader :threads

  # Verbose output?
  attr_reader :verbose

  # Use image_optim_pack
  attr_reader :pack

  # Skip workers with missing or problematic binaries
  attr_reader :skip_missing_workers

  # Allow lossy workers and optimizations
  attr_reader :allow_lossy

  # Cache directory
  attr_reader :cache_dir

  # Cache worker digests
  attr_reader :cache_worker_digests

  # Timeout in seconds for each image
  attr_reader :timeout

  # Initialize workers, specify options using worker underscored name:
  #
  # pass false to disable worker
  #
  #     ImageOptim.new(:pngcrush => false)
  #
  # or hash with options to worker
  #
  #     ImageOptim.new(:advpng => {:level => 3}, :optipng => {:level => 2})
  #
  # use :threads to set maximum number of parallel optimizers to run (passing
  # true or nil determines number of processors, false disables parallel
  # processing)
  #
  #     ImageOptim.new(:threads => 8)
  #
  # use :nice to specify optimizers nice level (true or nil makes it 10, false
  # makes it 0)
  #
  #     ImageOptim.new(:nice => 20)
  def initialize(options = {})
    config = Config.new(options)
    @verbose = config.verbose
    $stderr << "config:\n#{config.to_s.gsub(/^/, '  ')}" if verbose

    %w[
      nice
      threads
      pack
      skip_missing_workers
      allow_lossy
      cache_dir
      cache_worker_digests
      timeout
    ].each do |name|
      instance_variable_set(:"@#{name}", config.send(name))
      $stderr << "#{name}: #{send(name)}\n" if verbose
    end

    @bin_resolver = BinResolver.new(self)

    $stderr << "PATH: #{@bin_resolver.env_path}\n" if verbose

    @workers_by_format = Worker.create_all_by_format(self) do |klass|
      config.for_worker(klass)
    end

    @cache = Cache.new(self, @workers_by_format)

    log_workers_by_format if verbose

    config.assert_no_unused_options!
  end

  # Get workers for image
  def workers_for_image(path)
    @workers_by_format[Path.convert(path).image_format]
  end

  # Optimize one file, return new path as OptimizedPath or nil if
  # optimization failed
  def optimize_image(original)
    original = Path.convert(original)
    return unless (workers = workers_for_image(original))

    optimized = @cache.fetch(original) do
      timer = timeout && Timer.new(timeout)
      minimum_gain = minimum_worker_gain(original)
      previous_size = original.size

      Handler.for(original) do |handler|
        begin
          workers.each do |worker|
            stop_after_worker = false

            handler.process do |src, dst|
              optimized_by_worker = worker.optimize(src, dst, timeout: timer)

              if optimized_by_worker && (dst_size = dst.size?)
                current_gain = previous_size - dst_size
                stop_after_worker = worker.run_order > 0 &&
                  current_gain < minimum_gain
                previous_size = dst_size
              end

              optimized_by_worker
            end

            break if stop_after_worker
          end
        rescue Errors::TimeoutExceeded
          handler.result
        end
      end
    end

    return unless optimized

    OptimizedPath.new(optimized, original)
  end

  # Optimize one file in place, return original as OptimizedPath or nil if
  # optimization failed
  def optimize_image!(original)
    original = Path.convert(original)
    return unless (result = optimize_image(original))

    result.replace(original)
    OptimizedPath.new(original, result.original_size)
  end

  # Optimize image data, return new data or nil if optimization failed
  def optimize_image_data(original_data)
    format = ImageMeta.format_for_data(original_data)
    return unless format

    Path.temp_file %W[image_optim .#{format}] do |temp|
      temp.binmode
      temp.write(original_data)
      temp.close

      if (result = optimize_image(temp.path))
        result.binread
      end
    end
  end

  def benchmark_image(original)
    src = Path.convert(original)
    return unless (workers = workers_for_image(src))

    dst = src.temp_path
    begin
      workers.map do |worker|
        start = ElapsedTime.now
        worker.optimize(src, dst)
        BenchmarkResult.new(src, dst, ElapsedTime.now - start, worker)
      end
    ensure
      dst.unlink
    end
  end

  # Optimize multiple images
  # if block given yields path and result for each image and returns array of
  # yield results
  # else return array of path and result pairs
  def optimize_images(paths, &block)
    run_method_for(paths, :optimize_image, &block)
  end

  # Optimize multiple images in place
  # if block given yields path and result for each image and returns array of
  # yield results
  # else return array of path and result pairs
  def optimize_images!(paths, &block)
    run_method_for(paths, :optimize_image!, &block)
  end

  # Optimize multiple image datas
  # if block given yields original and result for each image data and returns
  # array of yield results
  # else return array of path and result pairs
  def optimize_images_data(datas, &block)
    run_method_for(datas, :optimize_image_data, &block)
  end

  def benchmark_images(paths, &block)
    run_method_for(paths, :benchmark_image, &block)
  end

  class << self
    # Optimization methods with default options
    def method_missing(method, *args, &block)
      if optimize_image_method?(method)
        new.send(method, *args, &block)
      else
        super
      end
    end

    def respond_to_missing?(method, include_private = false)
      optimize_image_method?(method) || super
    end

    # Version of image_optim gem spec loaded
    def version
      Gem.loaded_specs['image_optim'].version.to_s
    rescue
      'DEV'
    end

    # Full version of image_optim
    def full_version
      "image_optim v#{version}"
    end

  private

    def optimize_image_method?(method)
      method_defined?(method) && method.to_s =~ /^optimize_image/
    end
  end

  # Are there workers for file at path?
  def optimizable?(path)
    !!workers_for_image(path)
  end

  # Check existance of binary, create symlink if ENV contains path for key
  # XXX_BIN where XXX is upper case bin name
  def resolve_bin!(bin)
    @bin_resolver.resolve!(bin)
  end

  # Join resolve_dir, default path and vendor path for PATH environment variable
  def env_path
    @bin_resolver.env_path
  end

private

  def log_workers_by_format
    $stderr << "Workers by format:\n"
    @workers_by_format.each do |format, workers|
      $stderr << "#{format}:\n"
      workers.each do |worker|
        $stderr << "  #{worker.class.bin_sym}:\n"
        worker.options.each do |name, value|
          $stderr << "    #{name}: #{value.inspect}\n"
        end
      end
    end
  end

  # Run method for each item in list
  # if block given yields item and result for item and returns array of yield
  # results
  # else return array of item and result pairs
  def run_method_for(list, method_name, &block)
    apply_threading(list).map do |item|
      result = send(method_name, item)
      if block
        yield item, result
      else
        [item, result]
      end
    end
  end

  # Apply threading if threading is allowed
  def apply_threading(enum)
    thread_count = adaptive_thread_count(enum)

    if thread_count > 1
      enum.in_threads(thread_count)
    else
      enum
    end
  end

  def adaptive_thread_count(enum)
    return threads if threads <= 1

    average_size = average_item_size(enum)
    return threads unless average_size

    if average_size < SMALL_IMAGE_SIZE
      [threads, SMALL_IMAGE_THREADS].min
    elsif average_size < MEDIUM_IMAGE_SIZE
      [threads, MEDIUM_IMAGE_THREADS].min
    else
      threads
    end
  end

  def average_item_size(enum)
    source = source_enum(enum)
    return unless source.respond_to?(:each)

    total = 0
    count = 0

    source.each do |item|
      size = item_size(item)
      next unless size

      total += size
      count += 1
    end

    total / count.to_f unless count.zero?
  end

  def source_enum(enum)
    if enum.instance_variable_defined?(:@enum)
      enum.instance_variable_get(:@enum)
    else
      enum
    end
  end

  def item_size(item)
    if item.respond_to?(:to_path) && File.file?(item.to_path)
      File.size(item.to_path)
    elsif item.is_a?(String) && File.file?(item)
      File.size(item)
    elsif item.respond_to?(:size)
      item.size
    elsif File.file?(item.to_s)
      File.size(item.to_s)
    end
  rescue SystemCallError
    nil
  end

  def minimum_worker_gain(original)
    original.size * MIN_WORKER_GAIN_RATIO
  end
end
