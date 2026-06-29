package com.sparrow.sample;

import com.sparrow.*;

/**
 * Standalone sample: Java → SparrowLib.dll JNI integration.
 *
 * Build & run (Windows):
 *   cd SparrowDll_CMake
 *   javac -d out java/com/sparrow/*.java java-sample/SampleRunner.java
 *   java -Djava.library.path=target\debug -cp out com.sparrow.sample.SampleRunner data/input/albano.json
 *
 * Linux/macOS:
 *   javac -d out java/com/sparrow/*.java java-sample/SampleRunner.java
 *   java -Djava.library.path=target/debug -cp out com.sparrow.sample.SampleRunner data/input/albano.json
 */
public class SampleRunner {

    public static void main(String[] args) {
        System.out.println("=== SparrowLib JNI Sample ===");
        System.out.println("Version: " + SparrowOptimizer.getVersion());
        System.out.println();

        String inputFile = args.length > 0 ? args[0] : "data/input/albano.json";
        String outputDir = args.length > 1 ? args[1] : "output";

        // ─── Test 1: Simple optimization ─────────────────────────
        System.out.println("── Test 1: Simple Optimization ──");
        System.out.println("  input  : " + inputFile);
        System.out.println("  output : " + outputDir);
        System.out.println("  time   : 30s | seed : 42");

        long t0 = System.currentTimeMillis();
        int result = SparrowOptimizer.optimize(inputFile, outputDir, 30L, 42L);
        double elapsed = (System.currentTimeMillis() - t0) / 1000.0;
        System.out.printf("  → code=%d, elapsed=%.1fs%n%n", result, elapsed);

        // ─── Test 2: Extended optimization ───────────────────────
        System.out.println("── Test 2: Extended Optimization ──");
        SparrowConfig config = SparrowOptimizer.getDefaultConfig();
        config.rngSeed = 12345L;
        config.exploration.timeLimitSecs = 20.0f;
        config.compression.timeLimitSecs = 10.0f;
        config.exploration.maxConseqFailedAttempts = 15;

        System.out.println("  exploration time : " + config.exploration.timeLimitSecs + "s");
        System.out.println("  compression time : " + config.compression.timeLimitSecs + "s");
        System.out.println("  seed             : " + config.rngSeed);

        t0 = System.currentTimeMillis();
        OptimizationResult optResult = SparrowOptimizer.optimizeEx(inputFile, outputDir, config);
        elapsed = (System.currentTimeMillis() - t0) / 1000.0;

        System.out.printf("  success     : %s%n", optResult.success);
        if (optResult.success) {
            System.out.printf("  strip width : %.4f%n", optResult.stripWidth);
            System.out.printf("  density     : %.2f%%%n", optResult.density * 100);
            System.out.printf("  items       : %d%n", optResult.totalItems);
            System.out.printf("  elapsed     : %.1fs%n", elapsed);
            if (optResult.outputJson != null) {
                System.out.printf("  output JSON : %d chars%n", optResult.outputJson.length());
            }
        } else {
            System.out.printf("  error code  : %d%n", optResult.errorCode);
            System.out.printf("  error msg   : %s%n", optResult.errorMessage);
        }
        System.out.println();

        // ─── Test 3: Multiple seeds ──────────────────────────────
        System.out.println("── Test 3: Multiple Seeds ──");
        System.out.printf("  %-6s %12s %10s %8s%n", "seed", "width", "density", "time");
        System.out.println("  " + "-".repeat(40));

        for (int seed = 1; seed <= 3; seed++) {
            config.rngSeed = seed;
            t0 = System.currentTimeMillis();
            optResult = SparrowOptimizer.optimizeEx(inputFile, outputDir, config);
            elapsed = (System.currentTimeMillis() - t0) / 1000.0;

            if (optResult.success) {
                System.out.printf("  %-6d %12.4f %9.2f%% %7.1fs%n",
                    seed, optResult.stripWidth, optResult.density * 100, elapsed);
            } else {
                System.out.printf("  %-6d %12s (err=%d)%n", seed, "FAILED", optResult.errorCode);
            }
        }

        // ─── Test 4: Real-time progress callback ─────────────────
        System.out.println("\n── Test 4: Progress Callback (default config ~30s) ──");
        final String[] PHASES = {"ExplFeas", "ExplInfeas", "ExplImprov", "CmprFeas", "Final"};

        OptimizationResult cbResult = SparrowOptimizer.optimizeWithProgress(
            inputFile, outputDir, config,
            (reportType, stripWidth, density, solutionJson) -> {
                String phase = (reportType >= 0 && reportType < PHASES.length)
                    ? PHASES[reportType] : "?";
                String jsonInfo = (solutionJson != null)
                    ? solutionJson.length() + " chars" : "-";
                System.out.printf("  [%-10s] width=%.2f  density=%.2f%%  json=%s%n",
                    phase, stripWidth, density * 100, jsonInfo);
            }
        );
        System.out.println("  → final success=" + cbResult.success);

        System.out.println("\nDone!");
    }
}
