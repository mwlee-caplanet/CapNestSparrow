package com.sparrow;

/**
 * Callback interface for receiving real-time optimization progress from the
 * native Sparrow engine. Implementations are invoked from the native layer
 * (via JNI) each time a new solution is reported during optimization.
 *
 * <p>This is a functional interface, so it can be used with a lambda:</p>
 * <pre>
 * SparrowOptimizer.optimizeWithProgress(input, output, config,
 *     (reportType, stripWidth, density) -&gt;
 *         System.out.printf("type=%d width=%.2f density=%.2f%%%n",
 *             reportType, stripWidth, density * 100));
 * </pre>
 */
@FunctionalInterface
public interface ProgressListener {

	/**
	 * Called whenever the optimizer reports a solution.
	 *
	 * @param reportType   0 = ExplFeas, 1 = ExplInfeas, 2 = ExplImproving,
	 *                     3 = CmprFeas, 4 = Final
	 * @param stripWidth   current strip width
	 * @param density      current fill ratio (0.0 ~ 1.0)
	 * @param solutionJson solution-only JSON for this report (strip_width + layout +
	 *                     density), or {@code null} for scalar-only reports
	 *                     (feasibility probes / throttled / emission disabled)
	 */
	void onProgress(int reportType, float stripWidth, float density, String solutionJson);
}
