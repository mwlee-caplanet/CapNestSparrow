package com.sparrow;

/**
 * Configuration for Sparrow optimizer.
 * Maps to native CSparrowConfig structure.
 */
public class SparrowConfig {
	public long rngSeed = 0; // 0 = random
	public float polySimplTolerance = -1.0f; // negative = None
	public float minItemSeparation = -1.0f;
	public float narrowConcavityCutoffDist = -1.0f;
	public float narrowConcavityCutoffArea = -1.0f;

	public ExplorationConfig exploration = new ExplorationConfig();
	public CompressionConfig compression = new CompressionConfig();

	/**
	 * Separator configuration.
	 */
	public static class SeparatorConfig {
		public int iterNoImprvLimit = 300;
		public int strikeLimit = 5;
		public int nWorkers = 4;
		public int nContainerSamples = 100;
		public int nFocussedSamples = 50;
		public int nCoordDescents = 5;
	}

	/**
	 * Exploration phase configuration.
	 */
	public static class ExplorationConfig {
		public float shrinkStep = 0.002f;
		public float timeLimitSecs = 24.0f;
		public int maxConseqFailedAttempts = 10; // -1 = None
		public float solutionPoolDistributionStdDev = 0.3f;
		public float largeItemChAreaCutoffPercentile = 0.7f;
		public SeparatorConfig separator = new SeparatorConfig();
	}

	/**
	 * Compression phase configuration.
	 */
	public static class CompressionConfig {
		public float shrinkRangeMin = 0.001f;
		public float shrinkRangeMax = 0.00001f;
		public float timeLimitSecs = 6.0f;
		public int shrinkDecayStrategy = 0; // 0 = TimeBased, 1 = FailureBased
		public float shrinkDecayRatio = 0.9f;
		public SeparatorConfig separator = new SeparatorConfig();
	}
}
