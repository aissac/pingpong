package com.polybot.hft.polymarket.strategy.config;

import com.polybot.hft.config.HftProperties;

import java.math.BigDecimal;
import java.time.Instant;
import java.util.List;

/**
 * Complete configuration for the Gabagool strategy.
 * Grouped into logical sub-configurations for readability.
 */
public record GabagoolConfig(
        boolean enabled,
        TimingConfig timing,
        SizingConfig sizing,
        BankrollConfig bankroll,
        CompleteSetConfig completeSet,
        TakerConfig taker,
        DirectionalConfig directional,
        List<GabagoolMarketConfig> markets
) {

    public static GabagoolConfig defaults() {
        return new GabagoolConfig(
                false,
                TimingConfig.defaults(),
                SizingConfig.defaults(),
                BankrollConfig.defaults(),
                CompleteSetConfig.defaults(),
                TakerConfig.defaults(),
                DirectionalConfig.defaults(),
                List.of()
        );
    }

    /**
     * Build from HftProperties.Gabagool for backwards compatibility.
     */
    public static GabagoolConfig from(HftProperties.Gabagool cfg) {
        TimingConfig timing = new TimingConfig(
                cfg.refreshMillis(),
                cfg.minReplaceMillis(),
                cfg.minSecondsToEnd(),
                cfg.maxSecondsToEnd()
        );

        SizingConfig sizing = new SizingConfig(
                cfg.quoteSize(),
                cfg.quoteSizeBankrollFraction(),
                cfg.improveTicks()
        );

        BankrollConfig bankroll = new BankrollConfig(
                cfg.bankrollUsd(),
                cfg.bankrollMode(),
                cfg.bankrollRefreshMillis(),
                cfg.dynamicSizingEnabled(),
                cfg.dynamicSizingMinMultiplier(),
                cfg.dynamicSizingMaxMultiplier(),
                cfg.bankrollSmoothingAlpha(),
                cfg.bankrollMinThreshold(),
                cfg.bankrollTradingFraction(),
                cfg.maxOrderBankrollFraction(),
                cfg.maxTotalBankrollFraction()
        );

        CompleteSetConfig completeSet = new CompleteSetConfig(
                cfg.completeSetMinEdge(),
                cfg.completeSetMaxSkewTicks(),
                cfg.completeSetImbalanceSharesForMaxSkew(),
                cfg.completeSetTopUpEnabled(),
                cfg.completeSetTopUpSecondsToEnd(),
                cfg.completeSetTopUpMinShares(),
                cfg.completeSetFastTopUpEnabled(),
                cfg.completeSetFastTopUpMinShares(),
                cfg.completeSetFastTopUpMinSecondsAfterFill(),
                cfg.completeSetFastTopUpMaxSecondsAfterFill(),
                cfg.completeSetFastTopUpCooldownMillis(),
                cfg.completeSetFastTopUpMinEdge()
        );

        TakerConfig taker = new TakerConfig(
                cfg.takerModeEnabled(),
                cfg.takerModeMaxEdge(),
                cfg.takerModeMaxSpread()
        );

        DirectionalConfig directional = new DirectionalConfig(
                cfg.directionalEnabled() != null && cfg.directionalEnabled(),
                cfg.directionalMinEdge() != null ? cfg.directionalMinEdge() : 0.05,
                cfg.directionalMomentumTicks() != null ? cfg.directionalMomentumTicks() : 20,
                cfg.directionalHighConviction() != null ? cfg.directionalHighConviction() : 0.70,
                cfg.directionalSizeHigh() != null ? cfg.directionalSizeHigh() : 3.0,
                cfg.directionalSizeLow() != null ? cfg.directionalSizeLow() : 0.3,
                cfg.directionalMaxLossPerSlot() != null ? 
                    BigDecimal.valueOf(cfg.directionalMaxLossPerSlot()) : BigDecimal.TEN
        );

        List<GabagoolMarketConfig> markets = cfg.markets() == null ? List.of() :
                cfg.markets().stream()
                        .map(m -> {
                            Instant endTime = null;
                            if (m.endTime() != null && !m.endTime().isBlank()) {
                                try {
                                    endTime = Instant.parse(m.endTime());
                                } catch (Exception ignored) {}
                            }
                            return new GabagoolMarketConfig(m.slug(), m.upTokenId(), m.downTokenId(), endTime);
                        })
                        .toList();


        return new GabagoolConfig(cfg.enabled(), timing, sizing, bankroll, completeSet, taker, directional, markets);
    }

    // Convenience accessors for backwards compatibility
    public long refreshMillis() { return timing.refreshMillis(); }
    public long minReplaceMillis() { return timing.minReplaceMillis(); }
    public long minSecondsToEnd() { return timing.minSecondsToEnd(); }
    public long maxSecondsToEnd() { return timing.maxSecondsToEnd(); }

    public BigDecimal quoteSize() { return sizing.quoteSize(); }
    public double quoteSizeBankrollFraction() { return sizing.quoteSizeBankrollFraction(); }
    public int improveTicks() { return sizing.improveTicks(); }

    public BigDecimal bankrollUsd() { return bankroll.bankrollUsd(); }
    public HftProperties.BankrollMode bankrollMode() { return bankroll.bankrollMode(); }
    public long bankrollRefreshMillis() { return bankroll.bankrollRefreshMillis(); }
    public boolean dynamicSizingEnabled() { return bankroll.dynamicSizingEnabled(); }
    public double dynamicSizingMinMultiplier() { return bankroll.dynamicSizingMinMultiplier(); }
    public double dynamicSizingMaxMultiplier() { return bankroll.dynamicSizingMaxMultiplier(); }
    public double bankrollSmoothingAlpha() { return bankroll.bankrollSmoothingAlpha(); }
    public BigDecimal bankrollMinThreshold() { return bankroll.bankrollMinThreshold(); }
    public double bankrollTradingFraction() { return bankroll.bankrollTradingFraction(); }
    public double maxOrderBankrollFraction() { return bankroll.maxOrderBankrollFraction(); }
    public double maxTotalBankrollFraction() { return bankroll.maxTotalBankrollFraction(); }

    public double completeSetMinEdge() { return completeSet.minEdge(); }
    public int completeSetMaxSkewTicks() { return completeSet.maxSkewTicks(); }
    public BigDecimal completeSetImbalanceSharesForMaxSkew() { return completeSet.imbalanceSharesForMaxSkew(); }
    public boolean completeSetTopUpEnabled() { return completeSet.topUpEnabled(); }
    public long completeSetTopUpSecondsToEnd() { return completeSet.topUpSecondsToEnd(); }
    public BigDecimal completeSetTopUpMinShares() { return completeSet.topUpMinShares(); }
    public boolean completeSetFastTopUpEnabled() { return completeSet.fastTopUpEnabled(); }
    public BigDecimal completeSetFastTopUpMinShares() { return completeSet.fastTopUpMinShares(); }
    public long completeSetFastTopUpMinSecondsAfterFill() { return completeSet.fastTopUpMinSecondsAfterFill(); }
    public long completeSetFastTopUpMaxSecondsAfterFill() { return completeSet.fastTopUpMaxSecondsAfterFill(); }
    public long completeSetFastTopUpCooldownMillis() { return completeSet.fastTopUpCooldownMillis(); }
    public double completeSetFastTopUpMinEdge() { return completeSet.fastTopUpMinEdge(); }

    public boolean takerModeEnabled() { return taker.enabled(); }
    public double takerModeMaxEdge() { return taker.maxEdge(); }
    public BigDecimal takerModeMaxSpread() { return taker.maxSpread(); }

    // Directional config accessors
    public boolean directionalEnabled() { return directional.enabled(); }
    public double directionalMinEdge() { return directional.minEdge(); }
    public int directionalMomentumTicks() { return directional.momentumTicks(); }
    public double directionalHighConviction() { return directional.highConviction(); }
    public double directionalSizeHigh() { return directional.sizeHigh(); }
    public double directionalSizeLow() { return directional.sizeLow(); }
    public BigDecimal directionalMaxLossPerSlot() { return directional.maxLossPerSlot(); }
    public DirectionalConfig directional() { return directional; }

    public record GabagoolMarketConfig(
            String slug,
            String upTokenId,
            String downTokenId,
            Instant endTime
    ) {}
}
