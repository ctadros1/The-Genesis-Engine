#!/usr/bin/env python3
# Phase 14 simulated power at the world level (methodology review 7.12:
# closed-form power is rejected; the simulation runs the COMPLETE planned
# decision procedure over synthetic world-level outcomes whose parameters
# come from the pilot, never from confirmatory seeds).
#
# The C14.2 decision: D(s) = |dev7_A(s)| - |dev7_P(s)| per seed-paired
# world pair, direction increase, SESOI 10 milli absolute, bar 30 of 50,
# exact binomial tail at null rate 500 reported beside the count.
#
# Null model: D drawn by resampling the eight pilot paired values
# (phase14-physiology-pilot, seeds 14901..14908: [0,1,1,1,1,1,1,6] milli)
# with sign symmetrization - the pilot mean of +1.5 is treated as noise,
# not signal, because preference had not left neutral and the honest null
# is symmetric. Alternatives shift that resampled noise by a true effect.
# A Gaussian(0, 1.9) variant runs beside it as the misspecification check
# the review requires.
import random
import statistics
from math import comb

PILOT_VALUES = [0, 1, 1, 1, 1, 1, 1, 6]
SEEDS = 50
BAR = 30
SESOI = 10
TRIALS = 20_000
RNG = random.Random(0x14_0_14)


def binom_tail(k, n, p=0.5):
    return sum(comb(n, i) * p**i * (1 - p) ** (n - i) for i in range(k, n + 1))


def draw_null_resample():
    value = RNG.choice(PILOT_VALUES)
    return value if RNG.random() < 0.5 else -value


def draw_null_gaussian():
    return RNG.gauss(0.0, 1.9)


def power(effect, draw):
    passes = 0
    for _ in range(TRIALS):
        reaching = sum(1 for _ in range(SEEDS) if draw() + effect >= SESOI)
        if reaching >= BAR:
            passes += 1
    return passes / TRIALS


def main():
    print(f"# C14.2 simulated power: SESOI {SESOI} milli, bar {BAR}/{SEEDS}, "
          f"{TRIALS} trials per point")
    print(f"# bar-as-test validity: P(reaching >= {BAR} | null rate 500) = "
          f"{binom_tail(BAR, SEEDS):.4f}")
    print("# false-positive rate under each null model:")
    for name, draw in [
        ("resampled-pilot", draw_null_resample),
        ("gaussian-1.9", draw_null_gaussian),
    ]:
        print(f"#   {name}: {power(0, draw):.4f}")
    print("effect_milli resampled_power gaussian_power")
    for effect in [0, 2, 4, 6, 8, 10, 12, 15, 20]:
        print(f"{effect} {power(effect, draw_null_resample):.3f} "
              f"{power(effect, draw_null_gaussian):.3f}")
    print(f"# pilot paired values (|A|-|P|, cue 7, milli): {PILOT_VALUES}, "
          f"sd {statistics.stdev(PILOT_VALUES):.2f}")


if __name__ == "__main__":
    main()
