package com.sparrow.example;

import com.sparrow.*;

/**
 * Example usage of Sparrow JNI wrapper.
 */
public class SparrowExample {

	public static void main(String[] args) {
		System.out.println("=== Sparrow JNI Example ===");
		System.out.println("Version: " + SparrowOptimizer.getVersion());

		String inputFile = "data/input/albano.json";
		String outputDir = "output";

		// Example 1: Simple optimization
		System.out.println("\n--- Example 1: Simple Optimization ---");
		int result = SparrowOptimizer.optimize(
			inputFile,
			outputDir,
			30,  // 30 seconds
			42   // RNG seed
		);
		System.out.println("Result code: " + result);

		// Example 2: Extended optimization with custom configuration
		System.out.println("\n--- Example 2: Extended Optimization ---");
		SparrowConfig config = SparrowOptimizer.getDefaultConfig();
		config.rngSeed = 12345;
		config.exploration.timeLimitSecs = 20.0f;  // 20 seconds for exploration
		config.compression.timeLimitSecs = 10.0f;  // 10 seconds for compression
		config.exploration.maxConseqFailedAttempts = 15;

		OptimizationResult optResult = SparrowOptimizer.optimizeEx(
			inputFile,
			outputDir,
			config
		);

		System.out.println("Success: " + optResult.success);
		if (optResult.success) {
			System.out.println("Strip Width: " + optResult.stripWidth);
			System.out.println("Density: " + String.format("%.2f%%", optResult.density * 100));
			System.out.println("Total Items: " + optResult.totalItems);

			if (optResult.outputJson != null) {
				System.out.println("Output JSON length: " + optResult.outputJson.length());
			}
		} else {
			System.out.println("Error Code: " + optResult.errorCode);
			System.out.println("Error Message: " + optResult.errorMessage);
		}

		// Example 3: Multiple runs with different seeds
		System.out.println("\n--- Example 3: Multiple Runs ---");
		for (int seed = 1; seed <= 3; seed++) {
			config.rngSeed = seed;
			optResult = SparrowOptimizer.optimizeEx(inputFile, outputDir, config);
			if (optResult.success) {
				System.out.printf("Run %d: width=%.2f, density=%.2f%%\n",
					seed, optResult.stripWidth, optResult.density * 100);
			}
		}

		System.out.println("\nDone!");
	}
}
