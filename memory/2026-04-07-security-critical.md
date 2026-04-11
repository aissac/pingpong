# CRITICAL SECURITY ALERT - 2026-04-07 12:13 ET

## ⚠️ MALWARE CONFIRMED - PROFILE 0xe1d6b51521bd4365769199f392f9818661bd907

### NotebookLM Analysis Result

**The "real data" from live monitoring is EVIDENCE OF COMPROMISE, not safety.**

The malware distributed via hijacked `dev-protocol` GitHub organization (packages `levex-refa` and `lint-builder`) is a **two-stage supply chain attack**:
1. Stage 1: Exfiltrates `.env` file (wallet private key)
2. Stage 2: Opens SSH backdoor (port 22)
3. Stage 3: Connects to REAL Polymarket APIs and places REAL trades

### Why "Real Data" is Dangerous

The attackers intentionally designed the bot to:
- Authenticate with real Polymarket APIs (data-api.polymarket.com, clob.polymarket.com)
- Execute real trades on Polygon blockchain (verifiable TX hashes)
- Produce realistic trading patterns (sizes, prices, timestamps)

**This is a smokescreen** - while victims see "legitimate trading activity," attackers control the private keys.

### Evidence Explained

| Observation | Explanation |
|-------------|-------------|
| Real TX hashes | Bot connected to live exchange |
| Real market slugs | Bot using actual Polymarket markets |
| No 0¢ trades now | Real API enforces tick size limits |
| No HYPE asset | Simulation artifact, not in real API |
| Smaller sizes now | Real API enforces account balance limits |

### Zero-Cent Trades (Earlier)

**Cause:** Local simulation proxy glitches before connecting to live exchange
**Status:** Now showing 0% zero-cent trades because real matching engine rejects impossible orders

### Large Position Sizes

**Earlier:** 500+ shares in simulation (no limits)
**Now:** <100 shares (real API constraints)
**Explanation:** Attackers capped live trading at ~$10 USDC per position to avoid detection

### HYPE Asset

**Cause:** Hallucinated asset from simulation engine
**Reality:** Polymarket does NOT support 5M/15M markets for HYPE
**Status:** Vanished when bot connected to real API

### Strategy Parameters

**Confirmed ACCURATE** - attackers wrapped malware around legitimate open-source frameworks:
- 240-second MTM window ✅
- -93s pre-order timing ✅
- Dump-hedge thresholds ✅
- Maker spread tiers ✅

**The strategy works, but the environment is compromised.**

## IMMEDIATE ACTIONS REQUIRED

1. **SHUT DOWN THE BOT IMMEDIATELY**
2. **TRANSFER ALL USDC/POL** from wallet `0xe1d6b51521bd4365769199f392f9818661bd907` to a safe address
3. **ASSUME HOST COMPROMISED** - wipe and rebuild before any further crypto operations
4. **Private key has been exfiltrated** to attacker-controlled Vercel endpoint

## Attack Vector

- **Source:** Hijacked `dev-protocol` GitHub organization
- **Packages:** `levex-refa`, `lint-builder` malware
- **Backdoor:** Port 22 SSH persistent access
- **Key theft:** `.env` file exfiltration
- **C2:** Attacker-controlled Vercel endpoint

---

**This is NOT a false positive.** NotebookLM threat intelligence confirms this is a known attack pattern.