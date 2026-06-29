package com.sparrow;

/**
 * Optimization result from Sparrow optimizer.
 */
public class OptimizationResult {
	public boolean success = false;
	public float stripWidth = 0.0f;
	public float density = 0.0f;
	public int totalItems = 0;
	public int errorCode = 0;
	public String errorMessage = null;
	public String outputJson = null;

	@Override
	public String toString() {
		return "OptimizationResult{" +
				"success=" + success +
				", stripWidth=" + stripWidth +
				", density=" + (density * 100) + "%" +
				", totalItems=" + totalItems +
				", errorCode=" + errorCode +
				", errorMessage='" + errorMessage + '\'' +
				'}';
	}
}
