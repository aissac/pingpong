package com.polybot.hft.polymarket.strategy.config;

import java.math.BigDecimal;

/**
 * Directional asymmetric sizing configuration.
 * 
 * Goal: Small losses when wrong, large gains when right.
 * 
 * Sizing rules based on market price:
 * - Price 0.70+: SELL (fade overpriced) at 0.3x size → capped downside
 * - Price 0.55-0.70: HOLD (no edge, wait)
 * - Price 0.45-0.55: BUY (fair value) at 1.0x size
 * - Price 0.30-0.45: BUY (underpriced) at 2.0x size → asymmetric upside
 * - Price <0.30: SELL (fade extreme) at 0.3x size
 */
public record DirectionalConfig(
        boolean enabled,
        double minEdge,              // Minimum edge from fair value (0.50) required
        int momentumTicks,           // Number of price ticks to track for momentum
        double highConviction,       // Confidence threshold for large size (0.70 = 70%)
        double sizeHigh,             // Size multiplier when high conviction (3.0 = 3x)
        double sizeLow,              // Size multiplier when fading extremes (0.3 = 30%)
        BigDecimal maxLossPerSlot    // Cap loss per slot at $X
) {
    public static DirectionalConfig defaults() {
        return new DirectionalConfig(
                false,               // Disabled by default
                0.05,                // 5% min edge
                20,                  // Track 20 ticks
                0.70,                // 70% conviction threshold
                3.0,                 // 3x size when high conviction
                0.3,                 // 0.3x size when fading
                BigDecimal.valueOf(10) // $10 max loss per slot
        );
    }
    
    /**
     * Get size multiplier based on price level.
     * 
     * @param price Market price (0.00 - 1.00)
     * @return Size multiplier (0.3x, 1.0x, 2.0x, or 3.0x)
     */
    public double getSizeMultiplier(double price) {
        if (!enabled) {
            return 1.0;  // Use base size
        }
        
        if (price >= 0.70) {
            return sizeLow;  // Fade overpriced (small size)
        } else if (price >= 0.55) {
            return 0.0;  // No edge, skip trade
        } else if (price >= 0.45) {
            return 1.0;  // Fair value (base size)
        } else if (price >= 0.30) {
            return 2.0;  // Underpriced (2x size)
        } else {
            return sizeLow;  // Fade extreme (small size)
        }
    }
    
    /**
     * Determine action based on price.
     * 
     * @param price Market price
     * @return Action: "BUY", "SELL", or "HOLD"
     */
    public String getAction(double price) {
        if (!enabled) {
            return "HOLD";
        }
        
        if (price >= 0.70) {
            return "SELL";  // Fade overpriced
        } else if (price >= 0.55) {
            return "HOLD";  // No edge
        } else if (price >= 0.45) {
            return "BUY";   // Fair value
        } else if (price >= 0.30) {
            return "BUY";   // Underpriced
        } else {
            return "SELL";  // Fade extreme
        }
    }
}
