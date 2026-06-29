package com.sparrow;

/**
 * Java JNI wrapper for the Sparrow 2D strip packing optimizer.
 * 
 * Usage:
 * <pre>
 * // Simple optimization
 * int result = SparrowOptimizer.optimize(
 *     "data/input.json",
 *     "output",
 *     30,  // time in seconds
 *     42   // RNG seed
 * );
 * 
 * // Extended optimization with callbacks
 * SparrowConfig config = SparrowOptimizer.getDefaultConfig();
 * config.rngSeed = 12345;
 * config.exploration.timeLimitSecs = 20.0f;
 * config.compression.timeLimitSecs = 10.0f;
 * 
 * OptimizationResult result = SparrowOptimizer.optimizeEx(
 *     "data/input.json",
 *     "output",
 *     config
 * );
 * 
 * System.out.println("Success: " + result.success);
 * System.out.println("Strip width: " + result.stripWidth);
 * System.out.println("Density: " + (result.density * 100) + "%");
 * </pre>
 */
public class SparrowOptimizer {

	static {
		try {
			// Load the native library (built via CMake as SparrowLib / cargo --features jni)
			// On Windows: SparrowLib.dll
			// On Linux:   libSparrowLib.so
			// On macOS:   libSparrowLib.dylib
			System.loadLibrary("SparrowLib");
		} catch (UnsatisfiedLinkError e) {
			System.err.println("Failed to load SparrowLib library: " + e.getMessage());
			System.err.println("Make sure the native library is in the Java library path:");
			System.err.println("  -Djava.library.path=/path/to/library");
			throw e;
		}
	}

	/**
	 * Simple optimization with minimal parameters.
	 *
	 * @param inputPath Path to input JSON instance file
	 * @param outputDir Output directory for result files
	 * @param timeSecs  Time limit in seconds
	 * @param rngSeed   Random number generator seed (0 for random)
	 * @return 0 on success, non-zero on error
	 */
	public static native int optimize(
		String inputPath,
		String outputDir,
		long timeSecs,
		long rngSeed
	);

	/**
	 * Extended optimization with full configuration.
	 *
	 * @param inputPath Path to input JSON instance file
	 * @param outputDir Output directory for result files
	 * @param config    SparrowConfig with detailed settings
	 * @return OptimizationResult with results or error information
	 */
	public static native OptimizationResult optimizeEx(
		String inputPath,
		String outputDir,
		SparrowConfig config
	);

	/**
	 * Extended optimization with a real-time progress callback.
	 *
	 * <p>The {@code listener} is invoked from the native layer each time a new
	 * solution is reported during optimization. Pass {@code null} to run without
	 * a callback (equivalent to {@link #optimizeEx}).</p>
	 *
	 * @param inputPath Path to input JSON instance file
	 * @param outputDir Output directory for result files
	 * @param config    SparrowConfig with detailed settings
	 * @param listener  Progress callback (may be null)
	 * @return OptimizationResult with results or error information
	 */
	public static native OptimizationResult optimizeWithProgress(
		String inputPath,
		String outputDir,
		SparrowConfig config,
		ProgressListener listener
	);

	/**
	 * Get the default configuration.
	 *
	 * @return SparrowConfig with default values
	 */
	public static SparrowConfig getDefaultConfig() {
		return new SparrowConfig();
	}

	/**
	 * Get the library version string.
	 *
	 * @return Version string
	 */
	public static String getVersion() {
		return "Sparrow JNI v0.1.0";
	}
}
